//! Une Webview distante isolée, déplacée entre la modale et le shell détaché.
use crate::spotlight::{navigate_with_referer, navigation_allowed, open_external, video_urls};
use crate::spotlight_media::{MediaLifecycle, MediaState, MediaStatus};
use crate::spotlight_model::{selected_segment, transition, SpotlightAction, SpotlightState};
use serde::Deserialize;
use tauri::{Emitter, Manager};

const PLAYER: &str = "skin-spotlight-player";
const CONTROLS: &str = "skin-spotlight-controls";
#[derive(Default)]
pub struct ViewerRuntime(pub tokio::sync::Mutex<ViewerInner>);
#[derive(Default)]
pub struct ViewerInner {
    session: SpotlightState,
    media: MediaLifecycle,
    shell_generation: Option<u64>,
}
fn local(label: &str) -> bool {
    matches!(label, "main" | CONTROLS)
}
fn broadcast(app: &tauri::AppHandle, session: &SpotlightState) {
    for label in ["main", CONTROLS] {
        let _ = app.emit_to(
            tauri::EventTarget::webview(label),
            "skin-spotlight-state",
            session,
        );
    }
}
fn broadcast_media(app: &tauri::AppHandle, state: &MediaState) {
    for label in ["main", CONTROLS] {
        let _ = app.emit_to(
            tauri::EventTarget::webview(label),
            "skin-spotlight-media",
            state,
        );
    }
}
fn report_media(app: tauri::AppHandle, attempt: u64, status: MediaStatus) {
    tauri::async_runtime::spawn(async move {
        let runtime = app.state::<ViewerRuntime>();
        let mut inner = runtime.0.lock().await;
        if inner.media.update(attempt, status) {
            broadcast_media(&app, &inner.media.current());
        }
    });
}
#[tauri::command]
pub async fn skin_spotlight_media(
    window: tauri::Webview,
    runtime: tauri::State<'_, ViewerRuntime>,
) -> Result<MediaState, String> {
    if !local(window.label()) {
        return Err("spotlight_forbidden".into());
    }
    Ok(runtime.0.lock().await.media.current())
}
#[tauri::command]
pub async fn skin_spotlight_state(
    window: tauri::Webview,
    runtime: tauri::State<'_, ViewerRuntime>,
) -> Result<SpotlightState, String> {
    if !local(window.label()) {
        return Err("spotlight_forbidden".into());
    }
    Ok(runtime.0.lock().await.session.clone())
}
/// Une fermeture par la croix native doit aussi arrêter le média et réinitialiser l'interface.
fn detached_closed(app: tauri::AppHandle, generation: u64) {
    tauri::async_runtime::spawn(async move {
        let runtime = app.state::<ViewerRuntime>();
        let mut inner = runtime.0.lock().await;
        if current_shell(inner.shell_generation, generation, inner.session.detached) {
            inner.shell_generation = None;
            if let Some(player) = app.get_webview(PLAYER) {
                let _ = player.close();
            }
            inner.session.video = None;
            inner.session.detached = false;
            inner.session.selected = "full".into();
            inner.session.revision += 1;
            inner.media.invalidate();
            broadcast_media(&app, &inner.media.current());
            broadcast(&app, &inner.session);
        }
    });
}
#[tauri::command]
pub async fn skin_spotlight_control(
    app: tauri::AppHandle,
    window: tauri::Webview,
    runtime: tauri::State<'_, ViewerRuntime>,
    action: SpotlightAction,
    revision: u64,
) -> Result<SpotlightState, String> {
    if !local(window.label()) {
        return Err("spotlight_forbidden".into());
    }
    let mut inner = runtime.0.lock().await;
    let retry = matches!(action, SpotlightAction::Retry);
    let external = matches!(action, SpotlightAction::External);
    let next = transition(&inner.session, action, window.label(), revision)?;
    if external {
        let video = inner.session.video.as_ref().ok_or("spotlight_not_open")?;
        let segment = selected_segment(&inner.session);
        let (_, url) = video_urls(&video.video_id, segment.map(|s| s.start), None)?;
        tauri::async_runtime::spawn_blocking(move || open_external(&url))
            .await
            .map_err(|_| "spotlight_external_failed")??;
    }
    // Réessayer détruit aussi la session média bloquée, sans changer le passage.
    if next.video.is_none() || retry {
        if let Some(player) = app.get_webview(PLAYER) {
            player.close().map_err(|_| "spotlight_unavailable")?;
        }
        inner.media.invalidate();
        broadcast_media(&app, &inner.media.current());
    } else if next.detached != inner.session.detached {
        // Créer le shell avant de déplacer le média : un échec laisse la session actuelle intacte.
        if next.detached && app.get_window(CONTROLS).is_none() {
            let shell = tauri::WebviewWindowBuilder::new(
                &app,
                CONTROLS,
                tauri::WebviewUrl::App("index.html?spotlight".into()),
            )
            .title("SkinSpotlights")
            .inner_size(1000.0, 740.0)
            .min_inner_size(640.0, 520.0)
            .center()
            .visible(false)
            .on_navigation(|url| {
                matches!(url.scheme(), "tauri" | "http" | "https")
                    && matches!(url.host_str(), Some("localhost" | "tauri.localhost"))
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .build()
            .map_err(|_| "spotlight_unavailable")?;
            let generation = next.revision;
            let handle = app.clone();
            shell.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    detached_closed(handle.clone(), generation);
                }
            });
            inner.shell_generation = Some(generation);
        }
        let target = app
            .get_window(if next.detached { CONTROLS } else { "main" })
            .ok_or("spotlight_unavailable")?;
        if let Some(player) = app.get_webview(PLAYER) {
            player.hide().map_err(|_| "spotlight_unavailable")?;
            player
                .reparent(&target)
                .map_err(|_| "spotlight_unavailable")?;
        }
        target.show().map_err(|_| "spotlight_unavailable")?;
        target.set_focus().map_err(|_| "spotlight_unavailable")?;
    }
    inner.session = next;
    // Le média a déjà été déplacé avant de détruire son ancien conteneur.
    if !inner.session.detached {
        inner.shell_generation = None;
        if let Some(shell) = app.get_webview_window(CONTROLS) {
            let _ = shell.destroy();
        }
    }
    broadcast(&app, &inner.session);
    Ok(inner.session.clone())
}
#[derive(Deserialize)]
pub struct VideoBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
fn valid_bounds(b: &VideoBounds, width: f64, height: f64) -> bool {
    [b.x, b.y, b.width, b.height, width, height]
        .iter()
        .all(|v| v.is_finite())
        && b.x >= 0.0
        && b.y >= 0.0
        && b.width >= 200.0
        && b.height >= 200.0
        && b.x + b.width <= width + 1.0
        && b.y + b.height <= height + 1.0
}
#[tauri::command]
pub async fn skin_spotlight_layout(
    app: tauri::AppHandle,
    window: tauri::Webview,
    runtime: tauri::State<'_, ViewerRuntime>,
    revision: u64,
    bounds: VideoBounds,
) -> Result<(), String> {
    if !local(window.label()) {
        return Err("spotlight_forbidden".into());
    }
    let mut inner = runtime.0.lock().await;
    let host = if inner.session.detached {
        CONTROLS
    } else {
        "main"
    };
    if window.label() != host || revision != inner.session.revision || inner.session.video.is_none()
    {
        return Err("spotlight_stale".into());
    }
    let target = app.get_window(host).ok_or("spotlight_unavailable")?;
    let size = window
        .size()
        .map_err(|_| "spotlight_unavailable")?
        .to_logical::<f64>(target.scale_factor().map_err(|_| "spotlight_unavailable")?);
    if !valid_bounds(&bounds, size.width, size.height) {
        return Err("spotlight_invalid_bounds".into());
    }
    // Le DOM est relatif à la Webview locale, pas au contenu natif de la fenêtre.
    let origin = window
        .position()
        .map_err(|_| "spotlight_unavailable")?
        .to_logical::<f64>(target.scale_factor().map_err(|_| "spotlight_unavailable")?);
    let top_inset = dom_top_inset(&window).await?;
    let rect = tauri::Rect {
        position: tauri::LogicalPosition::new(origin.x + bounds.x, origin.y + top_inset + bounds.y)
            .into(),
        size: tauri::LogicalSize::new(bounds.width, bounds.height).into(),
    };
    let video = inner
        .session
        .video
        .as_ref()
        .ok_or("spotlight_unavailable")?;
    let segment = selected_segment(&inner.session);
    let (url, _) = video_urls(
        &video.video_id,
        segment.map(|s| s.start),
        segment.map(|s| s.end),
    )?;
    let navigate = inner.media.begin(&url);
    let attempt = inner.media.current().attempt;
    if navigate {
        // Chaque navigation a sa propre vue : callbacks et état média de l'ancienne
        // tentative ne peuvent pas contaminer la suivante. Détacher ne recharge pas.
        if let Some(player) = app.get_webview(PLAYER) {
            player.close().map_err(|_| "spotlight_unavailable")?;
        }
        broadcast_media(&app, &inner.media.current());
        let timeout_app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            report_media(timeout_app, attempt, MediaStatus::Slow);
        });
    }
    let player = match app.get_webview(PLAYER) {
        Some(player) => player,
        None => {
            let expected = url.clone();
            let handle = app.clone();
            target
                .add_child(
                    tauri::webview::WebviewBuilder::new(
                        PLAYER,
                        tauri::WebviewUrl::External(
                            "about:blank".parse().map_err(|_| "spotlight_unavailable")?,
                        ),
                    )
                    .incognito(true)
                    .on_navigation(|url| navigation_allowed(url.as_str()))
                    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                    .on_page_load(move |_, payload| {
                        // Finished confirme le document, pas la lecture du média YouTube.
                        if payload.url().as_str() == expected
                            && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                        {
                            report_media(handle.clone(), attempt, MediaStatus::Loaded);
                        }
                    }),
                    rect.position,
                    rect.size,
                )
                .map_err(|_| "spotlight_unavailable")?
        }
    };
    player
        .set_bounds(rect)
        .map_err(|_| "spotlight_unavailable")?;
    if navigate {
        let referer = format!("https://{}/", app.config().identifier.to_ascii_lowercase());
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let scheduled = player.with_webview(move |platform| {
            let _ = sender.send(navigate_with_referer(platform, &url, &referer));
        });
        let result = match scheduled {
            Ok(()) => tokio::time::timeout(std::time::Duration::from_secs(10), receiver)
                .await
                .map_err(|_| "spotlight_timeout".to_string())
                .and_then(|r| r.map_err(|_| "spotlight_unavailable".to_string()))
                .and_then(|r| r),
            Err(_) => Err("spotlight_unavailable".into()),
        };
        if let Err(error) = result {
            inner.media.update(attempt, MediaStatus::Failed);
            broadcast_media(&app, &inner.media.current());
            return Err(error);
        }
    }
    player
        .show()
        .map_err(|_| "spotlight_unavailable".to_string())
}

fn current_shell(active: Option<u64>, reported: u64, detached: bool) -> bool {
    detached && active == Some(reported)
}

// WebKit décale automatiquement le DOM sous une barre de titre opaque en mode
// FullSizeContentView ; la position native de la Webview n'inclut pas cet inset.
#[cfg(target_os = "macos")]
async fn dom_top_inset(webview: &tauri::Webview) -> Result<f64, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    webview
        .with_webview(move |platform| {
            let inset = unsafe {
                let view: &objc2_web_kit::WKWebView = &*platform.inner().cast();
                view.window().map_or(0.0, |window| {
                    if window
                        .styleMask()
                        .contains(objc2_app_kit::NSWindowStyleMask::FullSizeContentView)
                        && !window.titlebarAppearsTransparent()
                    {
                        view.convertRect_fromView(window.contentLayoutRect(), None)
                            .origin
                            .y
                            .max(0.0)
                    } else {
                        0.0
                    }
                })
            };
            let _ = send.send(inset);
        })
        .map_err(|_| "spotlight_unavailable")?;
    receive.await.map_err(|_| "spotlight_unavailable".into())
}
#[cfg(not(target_os = "macos"))]
async fn dom_top_inset(_webview: &tauri::Webview) -> Result<f64, String> {
    Ok(0.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn ignore_la_destruction_differee_d_un_ancien_shell() {
        assert!(!super::current_shell(Some(8), 7, true));
        assert!(super::current_shell(Some(8), 8, true));
        assert!(!super::current_shell(Some(8), 8, false));
    }
    use super::*;
    #[test]
    fn borne_la_video_dans_sa_fenetre_sans_accepter_nan_ni_zone_minuscule() {
        assert!(valid_bounds(
            &VideoBounds {
                x: 10.0,
                y: 70.0,
                width: 600.0,
                height: 340.0
            },
            800.0,
            600.0
        ));
        for b in [
            VideoBounds {
                x: f64::NAN,
                y: 70.0,
                width: 600.0,
                height: 340.0,
            },
            VideoBounds {
                x: -1.0,
                y: 70.0,
                width: 600.0,
                height: 340.0,
            },
            VideoBounds {
                x: 400.0,
                y: 70.0,
                width: 600.0,
                height: 340.0,
            },
            VideoBounds {
                x: 10.0,
                y: 70.0,
                width: 180.0,
                height: 180.0,
            },
        ] {
            assert!(!valid_bounds(&b, 800.0, 600.0));
        }
    }
    #[test]
    fn refuse_au_media_distant_les_commandes_du_shell() {
        assert!(local("main"));
        assert!(local(CONTROLS));
        assert!(!local(PLAYER));
        assert!(!local("game-overlay"));
    }
}
