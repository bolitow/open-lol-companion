//! Session transactionnelle : gestes temporaires, validation explicite, annulation sans écriture.
use super::*;
use olc_desktop_support::overlay_editor::{transform, Gesture, Panel};

pub(super) struct Editor {
    pub original: OverlayPreferences,
    frame: platform::GameWindow,
    in_game: bool,
    expires: Instant,
    gesture: Option<Drag>,
    minimum: (f64, f64),
}
struct Drag {
    start: (f64, f64),
    scale: f64,
    panel: Panel,
    kind: Gesture,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Move,
    Resize,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Action {
    Start {},
    Begin { session: u32, kind: Kind },
    Move { session: u32 },
    End { session: u32 },
    Commit { session: u32 },
    Cancel { session: u32 },
}
pub(super) fn cancel(runtime: &mut Runtime) {
    if let Some(editor) = runtime.editor.take() {
        runtime.public.preferences = editor.original;
    }
    runtime.public.edit_session = None;
    runtime.preview_until = None;
}
pub(super) fn check_context(app: &AppHandle, runtime: &mut Runtime) {
    let Some(editor) = &runtime.editor else {
        return;
    };
    let current = if editor.in_game {
        platform::foreground_game_rect()
    } else {
        preview_rect(app, runtime.public.preferences.monitor).map(|rect| platform::GameWindow {
            rect,
            overlay_level: 3,
        })
    };
    if context_lost(
        editor,
        current,
        runtime.live_ready,
        runtime.phase_active,
        Instant::now(),
    ) {
        cancel(runtime);
    }
}
fn context_lost(
    editor: &Editor,
    current: Option<platform::GameWindow>,
    ready: bool,
    active: bool,
    now: Instant,
) -> bool {
    now >= editor.expires
        || current != Some(editor.frame)
        || (editor.in_game && (!ready || !active))
        || (!editor.in_game && active)
}
fn valid_session(runtime: &Runtime, action: &Action) -> bool {
    token(action) == runtime.public.edit_session && runtime.editor.is_some()
}
fn start(app: &AppHandle, runtime: &mut Runtime) -> Result<(), &'static str> {
    if runtime.editor.is_some()
        || !runtime.public.available
        || runtime.public.preferences.exclusive_fullscreen
    {
        return Err("unavailable");
    }
    let game = if runtime.phase_active && runtime.live_ready && runtime.public.preferences.enabled {
        platform::foreground_game_rect()
    } else {
        None
    };
    // Ne pas substituer l'écran entier à une partie dont la fenêtre est introuvable.
    if runtime.phase_active && game.is_none() {
        return Err("unavailable");
    }
    let frame = game
        .or_else(|| {
            preview_rect(app, runtime.public.preferences.monitor).map(|rect| platform::GameWindow {
                rect,
                overlay_level: 3,
            })
        })
        .ok_or("unavailable")?;
    let display_scale = if frame.rect.physical {
        // Choisir le DPI cible avant de déplacer la fenêtre, pas son ancien écran.
        let window = app.get_webview_window(native::LABEL).ok_or("unavailable")?;
        let monitors = window.available_monitors().map_err(|_| "unavailable")?;
        // Le cadre peut chevaucher plusieurs écrans : garder le plus grand facteur
        // évite de couper les commandes quelle que soit la position du panneau.
        monitors
            .iter()
            .filter(|monitor| {
                let p = monitor.position();
                let size = monitor.size();
                let r = frame.rect;
                f64::from(p.x) < r.x + r.width
                    && f64::from(p.x) + f64::from(size.width) > r.x
                    && f64::from(p.y) < r.y + r.height
                    && f64::from(p.y) + f64::from(size.height) > r.y
            })
            .map(|m| m.scale_factor())
            .reduce(f64::max)
            .ok_or("unavailable")?
    } else {
        1.0
    };
    begin_session(runtime, frame, game.is_some(), display_scale)
}
fn begin_session(
    runtime: &mut Runtime,
    frame: platform::GameWindow,
    in_game: bool,
    display_scale: f64,
) -> Result<(), &'static str> {
    let minimum = (
        (160.0 * display_scale / frame.rect.width).clamp(0.1, 0.5),
        (120.0 * display_scale / frame.rect.height).clamp(0.1, 0.8),
    );
    let next_session = runtime.edit_counter.checked_add(1).ok_or("unavailable")?;
    let original = runtime.public.preferences.clone();
    let height = runtime
        .last_window
        .map(|w| w.rect.height / frame.rect.height)
        .unwrap_or(0.3)
        .clamp(0.1, 0.8);
    runtime.public.preferences.height = if original.height > 0.0 {
        original.height
    } else {
        height
    };
    runtime.public.preferences.width = original.width.max(minimum.0);
    runtime.public.preferences.height = runtime.public.preferences.height.max(minimum.1);
    runtime.public.preferences.x = original.x.min(1.0 - runtime.public.preferences.width);
    runtime.public.preferences.y = original.y.min(1.0 - runtime.public.preferences.height);
    runtime.edit_counter = next_session;
    runtime.public.edit_session = Some(runtime.edit_counter);
    runtime.editor = Some(Editor {
        original,
        frame,
        in_game,
        minimum,
        expires: Instant::now() + Duration::from_secs(180),
        gesture: None,
    });
    if !in_game {
        runtime.preview_until = Some(Instant::now() + Duration::from_secs(180));
    }
    Ok(())
}
fn cursor(app: &AppHandle) -> Result<((f64, f64), f64), &'static str> {
    let window = app.get_webview_window(native::LABEL).ok_or("unavailable")?;
    let position = window.cursor_position().map_err(|_| "unavailable")?;
    // Tao macOS multiplie NSEvent.mouseLocation par le facteur de l'écran principal.
    let scale = if cfg!(target_os = "macos") {
        window
            .primary_monitor()
            .map_err(|_| "unavailable")?
            .ok_or("unavailable")?
            .scale_factor()
    } else {
        1.0
    };
    if ![position.x, position.y, scale]
        .iter()
        .all(|x| x.is_finite())
        || scale <= 0.0
    {
        return Err("unavailable");
    }
    Ok(((position.x, position.y), scale))
}
fn token(action: &Action) -> Option<u32> {
    match action {
        Action::Start { .. } => None,
        Action::Begin { session, .. }
        | Action::Move { session }
        | Action::End { session }
        | Action::Commit { session }
        | Action::Cancel { session } => Some(*session),
    }
}
fn update(app: &AppHandle, runtime: &mut Runtime, action: Action) -> Result<(), &'static str> {
    check_context(app, runtime);
    if matches!(action, Action::Start { .. }) {
        return start(app, runtime);
    }
    if !valid_session(runtime, &action) {
        return Err("stale");
    }
    match action {
        Action::Start { .. } => unreachable!(),
        Action::Cancel { .. } => cancel(runtime),
        Action::Commit { .. } => {
            commit(runtime, |preferences| save(app, preferences))?;
        }
        Action::Begin { kind, .. } => {
            let (start, scale) = cursor(app)?;
            let p = &runtime.public.preferences;
            if let Some(editor) = &mut runtime.editor {
                editor.gesture = Some(Drag {
                    start,
                    scale,
                    panel: Panel {
                        x: p.x,
                        y: p.y,
                        width: p.width,
                        height: p.height,
                    },
                    kind: match kind {
                        Kind::Move => Gesture::Move,
                        Kind::Resize => Gesture::Resize,
                    },
                });
            }
        }
        Action::Move { .. } | Action::End { .. } => {
            let ((x, y), scale) = cursor(app)?;
            if let Some(editor) = &mut runtime.editor {
                if let Some(drag) = &editor.gesture {
                    if scale != drag.scale {
                        editor.gesture = None;
                        return Err("unavailable");
                    }
                    let rect = editor.frame.rect;
                    let mut next = transform(
                        drag.panel,
                        drag.kind,
                        (x - drag.start.0) / scale / rect.width,
                        (y - drag.start.1) / scale / rect.height,
                    )
                    .ok_or("unavailable")?;
                    next.width = next.width.max(editor.minimum.0);
                    next.height = next.height.max(editor.minimum.1);
                    next.x = next.x.min(1.0 - next.width);
                    next.y = next.y.min(1.0 - next.height);
                    let p = &mut runtime.public.preferences;
                    p.x = next.x;
                    p.y = next.y;
                    p.width = next.width;
                    p.height = next.height;
                }
                if matches!(action, Action::End { .. }) {
                    editor.gesture = None;
                }
            }
        }
    }
    Ok(())
}
fn commit(
    runtime: &mut Runtime,
    persist: impl FnOnce(&OverlayPreferences) -> Result<(), ()>,
) -> Result<(), &'static str> {
    if !runtime.public.preferences.valid() {
        return Err("unavailable");
    }
    if persist(&runtime.public.preferences).is_err() {
        runtime.public.error = Some("storage");
        return Err("storage");
    }
    runtime.editor = None;
    runtime.public.edit_session = None;
    runtime.preview_until = None;
    runtime.public.error = None;
    Ok(())
}
pub async fn handle(
    app: AppHandle,
    window: tauri::WebviewWindow,
    action: Action,
) -> Result<OverlayState, &'static str> {
    if window.label() != "main" && window.label() != native::LABEL {
        return Err("unavailable");
    }
    if matches!(action, Action::Start { .. }) && window.label() != "main" {
        return Err("unavailable");
    }
    on_main(app, move |app, runtime| update(app, runtime, action)).await
}
pub(super) fn register_shortcuts(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };
    for shortcut in [
        Shortcut::new(Some(Modifiers::ALT), Code::KeyB),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Backquote),
    ] {
        let result = app
            .global_shortcut()
            .on_shortcut(shortcut, |app, _, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Ok(mut runtime) = handle.state::<Shared>().lock() {
                        if runtime.editor.is_some() {
                            cancel(&mut runtime)
                        } else if start(&handle, &mut runtime).is_err() {
                            runtime.public.error = Some("unavailable");
                        }
                        refresh(&handle, &mut runtime);
                    };
                });
            });
        if result.is_err() {
            if let Ok(mut r) = app.state::<Shared>().lock() {
                r.public.error = Some("unavailable");
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn le_contrat_refuse_une_action_ou_un_champ_inconnu() {
        assert!(serde_json::from_str::<Action>(r#"{"type":"move","session":3}"#).is_ok());
        assert!(serde_json::from_str::<Action>(r#"{"type":"move","session":3,"x":2}"#).is_err());
        assert!(serde_json::from_str::<Action>(r#"{"type":"start","session":3}"#).is_err());
    }
    fn runtime() -> Runtime {
        Runtime {
            public: OverlayState {
                material: OverlayMaterial::Solid,
                revision: 0,
                preferences: OverlayPreferences::default(),
                available: true,
                visible: false,
                preview: false,
                edit_session: None,
                error: None,
            },
            preview_until: None,
            live_ready: false,
            phase_active: false,
            last_window: None,
            last_opacity: None,
            content_height: 180.0,
            shortcut_available: true,
            editor: None,
            edit_counter: 0,
            last_editing: false,
        }
    }
    fn frame() -> platform::GameWindow {
        platform::GameWindow {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1366.0,
                height: 768.0,
                physical: true,
            },
            overlay_level: 3,
        }
    }
    #[test]
    fn annuler_restaure_sans_confondre_les_sessions() {
        let mut r = runtime();
        let before = serde_json::to_value(&r.public.preferences).unwrap();
        begin_session(&mut r, frame(), false, 1.5).unwrap();
        let first = r.public.edit_session;
        r.public.preferences.x = 0.4;
        cancel(&mut r);
        assert_eq!(serde_json::to_value(&r.public.preferences).unwrap(), before);
        assert!(r.editor.is_none() && r.public.edit_session.is_none() && r.preview_until.is_none());
        begin_session(&mut r, frame(), false, 1.5).unwrap();
        assert_ne!(first, r.public.edit_session);
    }
    #[test]
    fn echec_stockage_garde_le_brouillon_puis_validation_le_conserve() {
        let mut r = runtime();
        begin_session(&mut r, frame(), false, 1.5).unwrap();
        r.public.preferences.x = 0.4;
        assert_eq!(commit(&mut r, |_| Err(())), Err("storage"));
        assert!(r.editor.is_some() && r.public.edit_session.is_some());
        assert_eq!(r.public.preferences.x, 0.4);
        commit(&mut r, |p| {
            assert_eq!(p.x, 0.4);
            Ok(())
        })
        .unwrap();
        assert!(r.editor.is_none() && r.public.edit_session.is_none());
        assert_eq!(r.public.preferences.x, 0.4);
    }
    #[test]
    fn petites_tailles_dpi_gardent_les_commandes_accessibles() {
        for target_scale in [1.0, 1.5, 2.0] {
            let mut r = runtime();
            r.public.preferences.width = 0.1;
            r.public.preferences.height = 0.1;
            begin_session(&mut r, frame(), false, target_scale).unwrap();
            assert!(r.public.preferences.width * 1366.0 / target_scale >= 159.9);
            assert!(r.public.preferences.height * 768.0 / target_scale >= 119.9);
            assert!(r.public.preferences.valid());
        }
    }
    #[test]
    fn ignore_anciens_gestes_et_interrompt_perte_jeu_ou_expiration() {
        let mut r = runtime();
        begin_session(&mut r, frame(), true, 1.0).unwrap();
        let session = r.public.edit_session.unwrap();
        assert!(valid_session(&r, &Action::Move { session }));
        assert!(!valid_session(
            &r,
            &Action::Move {
                session: session + 1
            }
        ));
        let e = r.editor.as_ref().unwrap();
        assert!(!context_lost(e, Some(frame()), true, true, Instant::now()));
        assert!(context_lost(e, None, true, true, Instant::now()));
        assert!(context_lost(e, Some(frame()), false, true, Instant::now()));
        assert!(context_lost(e, Some(frame()), true, true, e.expires));
        cancel(&mut r);
        assert!(!valid_session(&r, &Action::End { session }));
    }
}
