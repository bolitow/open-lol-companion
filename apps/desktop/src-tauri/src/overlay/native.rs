use super::platform::GameWindow;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder,
};

pub const LABEL: &str = "game-overlay";
#[cfg(target_os = "macos")]
mod mac {
    use tauri_nspanel::tauri_panel;
    tauri_panel! { OverlayPanel { config: {can_become_key_window: false, can_become_main_window: false, is_floating_panel: true} } }
}
/// Appelée dans setup, sur le thread principal. Création unique, cachée, jamais focalisée.
pub fn create(app: &AppHandle) -> Result<super::OverlayMaterial, ()> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Open LoL Companion Overlay")
        .inner_size(360.0, 220.0)
        .visible(false)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .accept_first_mouse(true)
        .focused(false)
        .focusable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()
        .map_err(|_| ())?;
    if window.set_ignore_cursor_events(true).is_err() {
        let _ = window.destroy();
        return Err(());
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindowCollectionBehavior;
        use tauri_nspanel::{StyleMask, WebviewWindowExt};
        let result = super::glass::install(&window, || {
            let panel = window.to_panel::<mac::OverlayPanel>().map_err(|_| ())?;
            panel
                .add_style_mask(StyleMask::empty().nonactivating_panel().into())
                .map_err(|_| ())?;
            panel.set_collection_behavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
            panel.set_hides_on_deactivate(false);
            panel.set_ignores_mouse_events(true);
            panel.set_has_shadow(false);
            panel.set_level(3); // NSFloatingWindowLevel, sans fenêtre capturant les entrées.
            Ok(())
        });
        if result.is_err() {
            let _ = window.destroy();
            return Err(());
        }
        result.map(|glass| {
            if glass {
                super::OverlayMaterial::LiquidGlass
            } else {
                super::OverlayMaterial::Vibrancy
            }
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window.destroy();
        return Err(());
    }
    #[cfg(target_os = "windows")]
    Ok(super::OverlayMaterial::Solid)
}
/// Position, NSPanel et visibilité sont toujours commandés sur le thread principal.
pub fn apply(
    app: &AppHandle,
    target: Option<GameWindow>,
    opacity: f64,
    editing: bool,
) -> Result<(), ()> {
    let rect = target.map(|window| window.rect);
    let window = app.get_webview_window(LABEL).ok_or(())?;
    window.set_ignore_cursor_events(!editing).map_err(|_| ())?;
    if let Some(rect) = rect {
        if rect.physical {
            window
                .set_size(PhysicalSize::new(
                    rect.width.round() as u32,
                    rect.height.round() as u32,
                ))
                .map_err(|_| ())?;
            window
                .set_position(PhysicalPosition::new(
                    rect.x.round() as i32,
                    rect.y.round() as i32,
                ))
                .map_err(|_| ())?;
        } else {
            window
                .set_size(LogicalSize::new(rect.width, rect.height))
                .map_err(|_| ())?;
            window
                .set_position(LogicalPosition::new(rect.x, rect.y))
                .map_err(|_| ())?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        let panel = app.get_webview_panel(LABEL).map_err(|_| ())?;
        panel.set_ignores_mouse_events(!editing);
        panel.set_alpha_value(opacity);
        panel.set_level(target.map_or(3, |window| window.overlay_level) as _);
        if rect.is_some() {
            panel.show();
        } else {
            panel.hide();
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    let _ = opacity;
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SWP_SHOWWINDOW,
        };
        let hwnd = window.hwnd().map_err(|_| ())?.0;
        let flags = SWP_NOMOVE
            | SWP_NOSIZE
            | SWP_NOACTIVATE
            | if rect.is_some() {
                SWP_SHOWWINDOW
            } else {
                SWP_HIDEWINDOW | SWP_NOZORDER
            };
        // L'affichage ET le masquage passent par Win32 : Tao garde son état initial
        // "caché". Aucun SW_SHOW activant ni mélange avec window.hide().
        // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos
        if unsafe { SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags) } == 0 {
            Err(())
        } else {
            Ok(())
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(())
    }
}
