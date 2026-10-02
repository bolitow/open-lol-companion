use super::geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameWindow {
    pub rect: Rect,
    pub overlay_level: i64,
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn is_game_executable(name: &str, windows: bool) -> bool {
    if windows {
        name.eq_ignore_ascii_case("League of Legends.exe")
    } else {
        name == "LeagueofLegends"
    }
}

#[cfg(any(target_os = "macos", test))]
fn is_macos_game(name: &str, bundle: Option<&str>) -> bool {
    is_game_executable(name, false)
        && matches!(
            bundle,
            None | Some("com.riotgames.LeagueofLegends.GameClient")
        )
}

#[cfg(any(target_os = "macos", test))]
struct WindowCandidate {
    pid: i32,
    layer: i64,
    alpha: f64,
    rect: Rect,
}

#[cfg(any(target_os = "macos", test))]
fn select_game_window(
    windows: impl IntoIterator<Item = WindowCandidate>,
    pid: i32,
) -> Option<GameWindow> {
    windows
        .into_iter()
        .filter(|window| {
            let rect = window.rect;
            pid > 0
                && window.pid == pid
                // LoL macOS utilise aussi le niveau écran de veille (1000) en sans bordure.
                // Les niveaux de menus/palettes restent exclus. Cas vérifié en partie réelle.
                && matches!(window.layer, 0 | 1000)
                && window.alpha.is_finite()
                && window.alpha > 0.0
                && window.alpha <= 1.0
                && [rect.x, rect.y, rect.width, rect.height]
                    .iter()
                    .all(|value| value.is_finite())
                && rect.width > 0.0
                && rect.height > 0.0
                && (rect.width * rect.height).is_finite()
        })
        .max_by(|left, right| {
            (left.rect.width * left.rect.height).total_cmp(&(right.rect.width * right.rect.height))
        })
        .map(|window| GameWindow {
            rect: window.rect,
            overlay_level: (window.layer + 1).max(3),
        })
}

/// Appelé sur le thread principal : aucune lecture d'écran ni activation du jeu.
#[cfg(target_os = "macos")]
pub fn foreground_game_rect() -> Option<GameWindow> {
    use core_foundation::{
        array::CFArray,
        base::{CFType, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    };
    use core_graphics::{geometry::CGRect, window::*};
    use objc2_app_kit::NSWorkspace;

    objc2::rc::autoreleasepool(|_| {
        let workspace = NSWorkspace::sharedWorkspace();
        let running = workspace.frontmostApplication()?;
        let name = running.executableURL()?.lastPathComponent()?.to_string();
        let bundle = running.bundleIdentifier().map(|value| value.to_string());
        if !is_macos_game(&name, bundle.as_deref()) {
            return None;
        }
        let pid = running.processIdentifier();
        // CoreGraphics garantit un tableau de dictionnaires pour cette fonction.
        // La règle Create transfère au CFArray la libération de l'objet retourné.
        let raw = unsafe {
            CGWindowListCopyWindowInfo(
                kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
                kCGNullWindowID,
            )
        };
        if raw.is_null() {
            return None;
        }
        let windows: CFArray<CFDictionary<CFString, CFType>> =
            unsafe { CFArray::wrap_under_create_rule(raw) };
        // Ces clés constantes sont des CFString gérées par CoreGraphics.
        let (pid_key, layer_key, alpha_key, bounds_key) = unsafe {
            (
                CFString::wrap_under_get_rule(kCGWindowOwnerPID),
                CFString::wrap_under_get_rule(kCGWindowLayer),
                CFString::wrap_under_get_rule(kCGWindowAlpha),
                CFString::wrap_under_get_rule(kCGWindowBounds),
            )
        };
        let candidates = windows.iter().filter_map(|window| {
            let pid =
                i32::try_from(window.find(&pid_key)?.downcast::<CFNumber>()?.to_i64()?).ok()?;
            let layer = window.find(&layer_key)?.downcast::<CFNumber>()?.to_i64()?;
            let alpha = window.find(&alpha_key)?.downcast::<CFNumber>()?.to_f64()?;
            let bounds = window.find(&bounds_key)?.downcast::<CFDictionary>()?;
            let bounds = CGRect::from_dict_representation(&bounds)?;
            Some(WindowCandidate {
                pid,
                layer,
                alpha,
                rect: Rect {
                    x: bounds.origin.x,
                    y: bounds.origin.y,
                    width: bounds.size.width,
                    height: bounds.size.height,
                    physical: false,
                },
            })
        });
        let rect = select_game_window(candidates, pid)?;
        // Un changement d'application pendant l'énumération invalide ce relevé.
        if workspace.frontmostApplication()?.processIdentifier() != pid {
            return None;
        }
        Some(rect)
    })
}

#[cfg(target_os = "windows")]
pub fn foreground_game_rect() -> Option<GameWindow> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            GetClientRect, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
        },
    };

    struct ProcessHandle(HANDLE);
    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // Seul un handle valide retourné par OpenProcess entre dans ce garde.
            unsafe { CloseHandle(self.0) };
        }
    }

    // Toutes les lectures vérifient le handle et le résultat avant d'utiliser les sorties.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() || IsWindowVisible(window) == 0 || IsIconic(window) != 0 {
            return None;
        }
        let mut pid = 0;
        if GetWindowThreadProcessId(window, &mut pid) == 0 || pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let process = ProcessHandle(process);
        let mut path = [0_u16; 32768];
        let mut length = path.len() as u32;
        if QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            path.as_mut_ptr(),
            &mut length,
        ) == 0
        {
            return None;
        }
        let executable = String::from_utf16(path.get(..length as usize)?).ok()?;
        let name = std::path::Path::new(&executable).file_name()?.to_str()?;
        if !is_game_executable(name, true) {
            return None;
        }
        let mut bounds = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut origin = POINT { x: 0, y: 0 };
        if GetClientRect(window, &mut bounds) == 0 || ClientToScreen(window, &mut origin) == 0 {
            return None;
        }
        let width = f64::from(bounds.right) - f64::from(bounds.left);
        let height = f64::from(bounds.bottom) - f64::from(bounds.top);
        if width <= 0.0 || height <= 0.0 || GetForegroundWindow() != window {
            return None;
        }
        Some(GameWindow {
            overlay_level: 0,
            rect: Rect {
                x: f64::from(origin.x),
                y: f64::from(origin.y),
                width,
                height,
                physical: true,
            },
        })
    }
}

/// Le moteur natif n'est pas pris en charge sur les autres OS de développement.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn foreground_game_rect() -> Option<GameWindow> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(pid: i32, layer: i64, width: f64, height: f64) -> WindowCandidate {
        WindowCandidate {
            pid,
            layer,
            alpha: 1.0,
            rect: Rect {
                x: -1920.0,
                y: 30.0,
                width,
                height,
                physical: false,
            },
        }
    }

    #[test]
    fn distingue_le_jeu_du_client_et_des_noms_ressemblants() {
        assert!(is_game_executable("LeagueofLegends", false));
        assert!(is_game_executable("League of Legends.exe", true));
        assert!(is_game_executable("LEAGUE OF LEGENDS.EXE", true));
        for name in [
            "LeagueClient",
            "LeagueClientUx.exe",
            "League of Legends.exe.bak",
            "League of Legends Helper",
            "league of legends",
        ] {
            assert!(!is_game_executable(name, false));
            assert!(!is_game_executable(name, true));
        }
        assert!(!is_game_executable("League of Legends.exe", false));
        assert!(!is_game_executable("League of Legends", true));
        assert!(!is_game_executable("League of Legends", false));
    }

    #[test]
    fn verifie_le_bundle_du_jeu_macos_lorsquil_est_disponible() {
        assert!(is_macos_game(
            "LeagueofLegends",
            Some("com.riotgames.LeagueofLegends.GameClient")
        ));
        assert!(is_macos_game("LeagueofLegends", None));
        assert!(!is_macos_game(
            "LeagueofLegends",
            Some("com.riotgames.LeagueofLegends.LeagueClientUx")
        ));
        assert!(!is_macos_game(
            "LeagueClientUx",
            Some("com.riotgames.LeagueofLegends.GameClient")
        ));
    }

    #[test]
    fn choisit_la_plus_grande_fenetre_visible_du_processus_actif() {
        let game = select_game_window(
            [
                candidate(42, 0, 800.0, 600.0),
                candidate(99, 0, 3840.0, 2160.0),
                candidate(42, 10, 3840.0, 2160.0),
                candidate(42, 0, 1920.0, 1080.0),
                candidate(42, 0, 0.0, 1200.0),
                WindowCandidate {
                    alpha: 0.0,
                    ..candidate(42, 0, 3840.0, 2160.0)
                },
            ],
            42,
        )
        .unwrap();
        assert_eq!(
            game.rect,
            Rect {
                x: -1920.0,
                y: 30.0,
                width: 1920.0,
                height: 1080.0,
                physical: false
            }
        );
    }

    #[test]
    fn refuse_un_cadre_absent_ou_non_fini() {
        assert!(select_game_window([], 42).is_none());
        assert!(select_game_window(
            [
                candidate(99, 0, 1920.0, 1080.0),
                candidate(42, 0, f64::INFINITY, 1080.0),
                candidate(42, 0, 1920.0, -1080.0),
                WindowCandidate {
                    alpha: f64::NAN,
                    ..candidate(42, 0, 1920.0, 1080.0)
                },
            ],
            42
        )
        .is_none());
    }

    #[test]
    fn reconnait_la_fenetre_sans_bordure_reelle_de_lol_sur_macos() {
        // Relevé de la personnalisée macOS : le jeu utilise le niveau 1000.
        let game = select_game_window([candidate(42, 1000, 2560.0, 1440.0)], 42);
        let game = game.unwrap();
        assert_eq!(game.overlay_level, 1001);
        assert_eq!(game.rect.width, 2560.0);
        let normal = select_game_window([candidate(42, 0, 2560.0, 1440.0)], 42).unwrap();
        assert_eq!(normal.rect, game.rect);
        assert_eq!(normal.overlay_level, 3);
        assert_ne!(normal, game); // Invalide le cache même sans changement de rectangle.
    }
}
