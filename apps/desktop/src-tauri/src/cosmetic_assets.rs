//! Images publiques à la demande, résolues uniquement depuis un instantané déclaré.
use olc_build_client::cosmetic_images::CosmeticImages;
use olc_catalog_cache::{cosmetics::Cosmetics, Cache};
use tauri::Manager;
#[derive(Default)]
pub struct CosmeticService {
    images: tokio::sync::OnceCell<CosmeticImages>,
}
fn request_path(path: &str) -> Option<(String, String, u32)> {
    let parts: Vec<_> = path.strip_prefix('/')?.split('/').collect();
    if parts.len() != 3
        || !olc_catalog_cache::valid_id(parts[0])
        || !["profile", "tile", "splash"].contains(&parts[1])
    {
        return None;
    }
    let id = parts[2].parse::<u32>().ok()?;
    if id.to_string() != parts[2] {
        return None;
    }
    Some((parts[0].into(), parts[1].into(), id))
}
/// Les métadonnées et l’image restent liées à l’instantané demandé, même après bascule.
pub fn protocol(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = request_path(request.uri().path());
    tauri::async_runtime::spawn(async move {
        let result = async {
            let (snapshot, kind, id) = path.ok_or(())?;
            let base = app.path().app_local_data_dir().map_err(|_| ())?;
            let catalog_root = base.join("desktop-catalog-v1");
            let url = tauri::async_runtime::spawn_blocking(move || {
                let cache = Cache::reader(&catalog_root).map_err(|_| ())?;
                let manifest = cache.manifest(&snapshot).map_err(|_| ())?;
                let metadata = Cosmetics::parse(
                    &cache.read(&snapshot, "cosmetics.json").map_err(|_| ())?,
                    &manifest.version,
                )
                .map_err(|_| ())?;
                metadata.image_url(&kind, id).ok_or(())
            })
            .await
            .map_err(|_| ())??;
            let state = app.state::<CosmeticService>();
            let images = state
                .images
                .get_or_try_init(|| async {
                    tauri::async_runtime::spawn_blocking(move || {
                        CosmeticImages::new(base.join("cosmetic-images-v1"))
                    })
                    .await
                    .map_err(|_| ())?
                    .map_err(|_| ())
                })
                .await?;
            images.load(&url).await.map_err(|_| ())
        }
        .await;
        let (status, body, media) = match result {
            Ok(image) => (tauri::http::StatusCode::OK, image.bytes, image.media_type),
            Err(()) => (
                tauri::http::StatusCode::NOT_FOUND,
                Vec::new(),
                "text/plain".into(),
            ),
        };
        let mut response = tauri::http::Response::new(body);
        *response.status_mut() = status;
        if let Ok(value) = tauri::http::HeaderValue::from_str(&media) {
            response
                .headers_mut()
                .insert(tauri::http::header::CONTENT_TYPE, value);
        }
        response.headers_mut().insert(
            tauri::http::header::CACHE_CONTROL,
            tauri::http::HeaderValue::from_static(if status.is_success() {
                "private, max-age=86400"
            } else {
                "no-store"
            }),
        );
        responder.respond(response);
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_declares_uniquement() {
        let id = "a".repeat(64);
        assert_eq!(
            request_path(&format!("/{id}/profile/0")),
            Some((id.clone(), "profile".into(), 0))
        );
        assert!(request_path(&format!("/{id}/splash/103001")).is_some());
        for p in [
            format!("/{id}/evil/1"),
            format!("/{id}/tile/01"),
            format!("/{id}/tile/1/extra"),
            "/latest/profile/1".into(),
            format!("/{id}/tile/%2e%2e"),
        ] {
            assert!(request_path(&p).is_none(), "{p}");
        }
    }
}
