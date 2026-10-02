mod geometry;
#[cfg(target_os = "macos")]
mod glass;
mod native;
mod platform;
mod policy;

use geometry::Rect;
use lcu_connector::live::{LiveSession, LiveStatus};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayPreferences {
    pub enabled: bool,
    pub exclusive_fullscreen: bool,
    pub monitor: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub opacity: f64,
    pub locale: String,
}
impl Default for OverlayPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            exclusive_fullscreen: false,
            monitor: 0,
            x: 0.02,
            y: 0.18,
            width: 0.20,
            opacity: 0.9,
            locale: "fr".into(),
        }
    }
}
impl OverlayPreferences {
    fn valid(&self) -> bool {
        self.monitor < 16
            && (0.0..=0.9).contains(&self.x)
            && (0.0..=0.9).contains(&self.y)
            && (0.1..=0.5).contains(&self.width)
            && self.x + self.width <= 1.0
            && (0.2..=1.0).contains(&self.opacity)
            && matches!(self.locale.as_str(), "fr" | "en")
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayMaterial {
    Solid,
    // Variantes du contrat partagé, disponibles au rendu natif sur macOS uniquement.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Vibrancy,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    LiquidGlass,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverlayState {
    pub material: OverlayMaterial,
    pub revision: u32,
    pub preferences: OverlayPreferences,
    pub available: bool,
    pub visible: bool,
    pub preview: bool,
    pub error: Option<&'static str>,
}
pub(super) struct Runtime {
    public: OverlayState,
    preview_until: Option<Instant>,
    live_ready: bool,
    phase_active: bool,
    last_window: Option<platform::GameWindow>,
    last_opacity: Option<f64>,
    content_height: f64,
    shortcut_available: bool,
}
type Shared = Arc<Mutex<Runtime>>;
fn require_main(window: &tauri::WebviewWindow) -> Result<(), &'static str> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("unavailable")
    }
}
fn load(app: &AppHandle) -> (OverlayPreferences, Option<&'static str>) {
    let Ok(dir) = app.path().app_config_dir() else {
        return (OverlayPreferences::default(), Some("storage"));
    };
    match std::fs::read(dir.join("overlay.json")) {
        Ok(bytes) => match serde_json::from_slice::<OverlayPreferences>(&bytes)
            .ok()
            .filter(OverlayPreferences::valid)
        {
            Some(p) => (p, None),
            None => (OverlayPreferences::default(), Some("storage")),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (OverlayPreferences::default(), None),
        Err(_) => (OverlayPreferences::default(), Some("storage")),
    }
}
fn save(app: &AppHandle, p: &OverlayPreferences) -> Result<(), ()> {
    let dir = app.path().app_config_dir().map_err(|_| ())?;
    std::fs::create_dir_all(&dir).map_err(|_| ())?;
    let bytes = serde_json::to_vec_pretty(p).map_err(|_| ())?;
    std::fs::write(dir.join("overlay.tmp"), bytes).map_err(|_| ())?;
    std::fs::rename(dir.join("overlay.tmp"), dir.join("overlay.json")).map_err(|_| ())
}
fn preference_error(saved: Result<(), ()>, available: bool) -> Option<&'static str> {
    saved
        .err()
        .map(|_| "storage")
        .or_else(|| (!available).then_some("unavailable"))
}
fn publish(app: &AppHandle, runtime: &mut Runtime) {
    runtime.public.revision = runtime.public.revision.saturating_add(1);
    let _ = app.emit("overlay-state", runtime.public.clone());
}
fn preview_rect(app: &AppHandle, monitor: u32) -> Option<Rect> {
    let window = app.get_webview_window("main")?;
    let monitors = window.available_monitors().ok()?;
    let monitor = monitors.get(monitor as usize)?;
    let pos = monitor.position();
    let size = monitor.size();
    // Sur Mac, utiliser les points du moniteur cible avant de déplacer la fenêtre :
    // le facteur courant de la WebView peut encore être celui de l'ancien écran.
    let scale = if cfg!(target_os = "macos") {
        monitor.scale_factor()
    } else {
        1.0
    };
    Some(Rect {
        x: f64::from(pos.x) / scale,
        y: f64::from(pos.y) / scale,
        width: f64::from(size.width) / scale,
        height: f64::from(size.height) / scale,
        physical: !cfg!(target_os = "macos"),
    })
}
fn refresh(app: &AppHandle, runtime: &mut Runtime) {
    if !runtime.public.available {
        publish(app, runtime);
        return;
    }
    let now = Instant::now();
    if runtime.preview_until.is_some_and(|until| until <= now) {
        runtime.preview_until = None;
    }
    let preview = runtime.preview_until.is_some();
    let p = &runtime.public.preferences;
    let game =
        if runtime.public.available && p.enabled && !p.exclusive_fullscreen && runtime.live_ready {
            platform::foreground_game_rect()
        } else {
            None
        };
    let mode = policy::presentation(
        runtime.public.available,
        p.enabled,
        p.exclusive_fullscreen,
        runtime.live_ready && runtime.phase_active,
        game.is_some(),
        preview,
    );
    let frame = match mode {
        policy::Presentation::Hidden => None,
        policy::Presentation::Preview => {
            preview_rect(app, p.monitor).map(|rect| platform::GameWindow {
                rect,
                overlay_level: 3,
            })
        }
        policy::Presentation::Game => game,
    };
    let target_window = frame.and_then(|window| {
        let r = window.rect;
        let scale = if r.physical {
            app.get_webview_window(native::LABEL)?.scale_factor().ok()?
        } else {
            1.0
        };
        geometry::fit_content(r, p.x, p.y, p.width, runtime.content_height, scale)
            .ok()
            .map(|rect| platform::GameWindow { rect, ..window })
    });
    runtime.public.preview = preview;
    if (runtime.last_window != target_window || runtime.last_opacity != Some(p.opacity))
        && native::apply(app, target_window, p.opacity).is_err()
    {
        // Aucun affichage si adaptation native/click-through échoue ; fermeture en secours.
        let _ = native::apply(app, None, 1.0);
        if let Some(window) = app.get_webview_window(native::LABEL) {
            let _ = window.destroy();
        }
        runtime.public.available = false;
        runtime.public.visible = false;
        runtime.public.error = Some("unavailable");
    } else {
        runtime.public.visible = target_window.is_some();
        runtime.last_window = target_window;
        runtime.last_opacity = Some(p.opacity);
    }
    publish(app, runtime);
}
pub fn setup(app: &AppHandle) {
    let (preferences, error) = load(app);
    let native = native::create(app);
    let available = native.is_ok();
    let shared = Arc::new(Mutex::new(Runtime {
        public: OverlayState {
            material: native.unwrap_or(OverlayMaterial::Solid),
            revision: 0,
            preferences,
            available,
            visible: false,
            preview: false,
            error: if available {
                error
            } else {
                Some("unavailable")
            },
        },
        preview_until: None,
        live_ready: false,
        phase_active: false,
        last_window: None,
        last_opacity: None,
        content_height: 180.0,
        shortcut_available: false,
    }));
    app.manage(shared);
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyO);
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let shared = handle.state::<Shared>();
                    if let Ok(mut runtime) = shared.lock() {
                        runtime.public.preferences.enabled = !runtime.public.preferences.enabled;
                        runtime.public.error = preference_error(
                            save(&handle, &runtime.public.preferences),
                            runtime.public.available && runtime.shortcut_available,
                        );
                        refresh(&handle, &mut runtime);
                    };
                });
            }
        });
    if let Ok(mut r) = app.state::<Shared>().lock() {
        r.shortcut_available = result.is_ok();
        if !r.shortcut_available {
            r.public.error = Some("unavailable");
        }
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let app = handle.clone();
            if handle
                .run_on_main_thread(move || {
                    let shared = app.state::<Shared>();
                    if let Ok(mut runtime) = shared.lock() {
                        refresh(&app, &mut runtime);
                    };
                })
                .is_err()
            {
                break;
            }
        }
    });
}
pub fn lcu_changed(app: &AppHandle, session: &lcu_connector::LcuSession) {
    if let Some(shared) = app.try_state::<Shared>() {
        if let Ok(mut runtime) = shared.lock() {
            runtime.phase_active = session.connected
                && session
                    .phase
                    .is_some_and(lcu_connector::GameflowPhase::is_in_game);
        }
    }
}
pub fn live_changed(app: &AppHandle, live: &LiveSession) {
    if let Some(shared) = app.try_state::<Shared>() {
        if let Ok(mut runtime) = shared.lock() {
            runtime.live_ready = live.status == LiveStatus::Ready;
        }
    }
}
#[tauri::command]
pub fn overlay_state(state: tauri::State<'_, Shared>) -> Result<OverlayState, &'static str> {
    state
        .lock()
        .map(|r| r.public.clone())
        .map_err(|_| "unavailable")
}
async fn on_main(
    app: AppHandle,
    update: impl FnOnce(&AppHandle, &mut Runtime) -> Result<(), &'static str> + Send + 'static,
) -> Result<OverlayState, &'static str> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let shared = handle.state::<Shared>();
        let result = shared
            .lock()
            .map_err(|_| "unavailable")
            .and_then(|mut runtime| {
                update(&handle, &mut runtime)?;
                refresh(&handle, &mut runtime);
                Ok(runtime.public.clone())
            });
        let _ = tx.send(result);
    })
    .map_err(|_| "unavailable")?;
    rx.await.map_err(|_| "unavailable")?
}
#[tauri::command]
pub async fn overlay_configure(
    app: AppHandle,
    window: tauri::WebviewWindow,
    preferences: OverlayPreferences,
) -> Result<OverlayState, &'static str> {
    require_main(&window)?;
    if !preferences.valid() {
        return Err("unavailable");
    }
    on_main(app, move |app, runtime| {
        runtime.public.error = preference_error(
            save(app, &preferences),
            runtime.public.available && runtime.shortcut_available,
        );
        runtime.public.preferences = preferences;
        if runtime.public.preferences.exclusive_fullscreen {
            runtime.preview_until = None;
        }
        Ok(())
    })
    .await
}
fn change_locale(preferences: &mut OverlayPreferences, locale: &str) -> Result<(), &'static str> {
    if !matches!(locale, "fr" | "en") {
        return Err("unavailable");
    }
    preferences.locale = locale.into();
    Ok(())
}
#[tauri::command]
pub async fn overlay_locale(
    app: AppHandle,
    window: tauri::WebviewWindow,
    locale: String,
) -> Result<OverlayState, &'static str> {
    require_main(&window)?;
    on_main(app, move |app, runtime| {
        change_locale(&mut runtime.public.preferences, &locale)?;
        runtime.public.error = preference_error(
            save(app, &runtime.public.preferences),
            runtime.public.available && runtime.shortcut_available,
        );
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn overlay_content_height(
    app: AppHandle,
    window: tauri::WebviewWindow,
    height: f64,
) -> Result<(), &'static str> {
    if window.label() != native::LABEL || !(48.0..=1200.0).contains(&height) {
        return Err("unavailable");
    }
    on_main(app, move |_, runtime| {
        runtime.content_height = height;
        Ok(())
    })
    .await
    .map(|_| ())
}
#[tauri::command]
pub async fn overlay_preview(
    app: AppHandle,
    window: tauri::WebviewWindow,
    enabled: bool,
) -> Result<OverlayState, &'static str> {
    require_main(&window)?;
    on_main(app, move |app, runtime| {
        if enabled
            && (!runtime.public.available
                || runtime.public.preferences.exclusive_fullscreen
                || preview_rect(app, runtime.public.preferences.monitor).is_none())
        {
            return Err("unavailable");
        }
        runtime.preview_until = enabled.then(|| Instant::now() + Duration::from_secs(30));
        Ok(())
    })
    .await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sauvegarder_ne_masque_pas_un_raccourci_indisponible() {
        assert_eq!(preference_error(Ok(()), false), Some("unavailable"));
        assert_eq!(preference_error(Err(()), false), Some("storage"));
        assert_eq!(preference_error(Ok(()), true), None);
    }
    #[test]
    fn changer_la_langue_ne_modifie_pas_les_reglages_du_panneau() {
        let mut preferences = OverlayPreferences {
            enabled: true,
            x: 0.4,
            ..Default::default()
        };
        change_locale(&mut preferences, "en").unwrap();
        assert_eq!(preferences.locale, "en");
        assert!(preferences.enabled);
        assert_eq!(preferences.x, 0.4);
        assert!(change_locale(&mut preferences, "xx").is_err());
        assert_eq!(preferences.locale, "en");
    }
    #[test]
    fn reglage_hors_cadre_ou_non_fini_refuse() {
        let mut p = OverlayPreferences::default();
        assert!(p.valid());
        p.x = 0.9;
        assert!(!p.valid());
        p.x = f64::NAN;
        assert!(!p.valid());
        p = OverlayPreferences::default();
        p.width = 0.01;
        assert!(!p.valid());
        p = OverlayPreferences::default();
        p.opacity = 0.0;
        assert!(!p.valid());
    }
}
