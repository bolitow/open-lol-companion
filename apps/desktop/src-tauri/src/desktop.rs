//! Réglages système : les appels OS et les fichiers restent dans Rust.
use olc_desktop_support::{load_preferences, save_preferences, Locale, NativePreferences};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::ManagerExt;

pub struct DesktopState {
    preferences: Mutex<NativePreferences>,
    path: PathBuf,
    pub close_to_tray: AtomicBool,
    pub tray_available: AtomicBool,
    storage_error: AtomicBool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSettings {
    close_to_tray: bool,
    autostart_enabled: Option<bool>,
    tray_available: bool,
    storage_error: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopSettingKey {
    CloseToTray,
    AutostartEnabled,
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn labels(locale: Locale) -> [&'static str; 3] {
    match locale {
        Locale::Fr => ["Ouvrir", "Réglages", "Quitter"],
        Locale::En => ["Open", "Settings", "Quit"],
    }
}
fn menu(app: &AppHandle, locale: Locale) -> tauri::Result<Menu<tauri::Wry>> {
    let [open, settings, quit] = labels(locale);
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", open, true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", settings, true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", quit, true, None::<&str>)?,
        ],
    )
}
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let path = app.path().app_config_dir()?.join("desktop-settings.json");
    let loaded = load_preferences(&path);
    let storage_error = loaded.is_err();
    let preferences = loaded.unwrap_or_default();
    app.manage(DesktopState {
        preferences: Mutex::new(preferences),
        path,
        close_to_tray: AtomicBool::new(preferences.close_to_tray),
        tray_available: AtomicBool::new(false),
        storage_error: AtomicBool::new(storage_error),
    });
    // Une erreur du tray ne doit jamais rendre la fenêtre inaccessible.
    if let (Some(icon), Ok(menu)) = (app.default_window_icon(), menu(app, preferences.locale)) {
        let result = TrayIconBuilder::with_id("companion")
            .icon(icon.clone())
            .tooltip("Open LoL Companion")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "open" => show_main(app),
                "settings" => {
                    show_main(app);
                    let _ = app.emit("open-settings", ());
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                ) {
                    show_main(tray.app_handle());
                }
            })
            .build(app);
        app.state::<DesktopState>()
            .tray_available
            .store(result.is_ok(), Ordering::Relaxed);
    }
    let tray = app
        .state::<DesktopState>()
        .tray_available
        .load(Ordering::Relaxed);
    if olc_desktop_support::should_start_hidden(
        std::env::args().any(|arg| arg == "--autostart"),
        tray,
    ) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    }
    Ok(())
}
fn snapshot(app: &AppHandle) -> DesktopSettings {
    let state = app.state::<DesktopState>();
    DesktopSettings {
        close_to_tray: state.close_to_tray.load(Ordering::Relaxed),
        autostart_enabled: app.autolaunch().is_enabled().ok(),
        tray_available: state.tray_available.load(Ordering::Relaxed),
        storage_error: state.storage_error.load(Ordering::Relaxed),
    }
}
#[tauri::command]
pub async fn desktop_settings(app: AppHandle) -> Result<DesktopSettings, &'static str> {
    tauri::async_runtime::spawn_blocking(move || snapshot(&app))
        .await
        .map_err(|_| "read_failed")
}
#[tauri::command]
pub async fn set_desktop_setting(
    app: AppHandle,
    key: DesktopSettingKey,
    value: bool,
) -> Result<DesktopSettings, &'static str> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<DesktopState>();
        let mut preferences = state.preferences.lock().map_err(|_| "unavailable")?;
        match key {
            DesktopSettingKey::CloseToTray => {
                if !state.tray_available.load(Ordering::Relaxed) {
                    return Err("unavailable");
                }
                let next = NativePreferences {
                    close_to_tray: value,
                    ..*preferences
                };
                save_preferences(&state.path, &next).map_err(|_| "write_failed")?;
                *preferences = next;
                state.close_to_tray.store(value, Ordering::Relaxed);
                state.storage_error.store(false, Ordering::Relaxed);
            }
            DesktopSettingKey::AutostartEnabled => {
                if value {
                    app.autolaunch().enable()
                } else {
                    app.autolaunch().disable()
                }
                .map_err(|_| "autostart_failed")?;
                if app.autolaunch().is_enabled().map_err(|_| "read_failed")? != value {
                    return Err("verification_failed");
                }
            }
        }
        Ok(snapshot(&app))
    })
    .await
    .map_err(|_| "unavailable")?
}
#[tauri::command]
pub async fn set_desktop_locale(app: AppHandle, locale: Locale) -> Result<(), &'static str> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<DesktopState>();
        let mut preferences = state.preferences.lock().map_err(|_| "unavailable")?;
        // Une synchronisation automatique ne remplace pas un fichier endommagé.
        if state.storage_error.load(Ordering::Relaxed) {
            return Err("read_failed");
        }
        if preferences.locale != locale {
            let next = NativePreferences {
                locale,
                ..*preferences
            };
            save_preferences(&state.path, &next).map_err(|_| "write_failed")?;
            *preferences = next;
        }
        Ok(())
    })
    .await
    .map_err(|_| "unavailable")??;
    if let Some(tray) = app.tray_by_id("companion") {
        tray.set_menu(Some(menu(&app, locale).map_err(|_| "unavailable")?))
            .map_err(|_| "unavailable")?;
    }
    Ok(())
}
