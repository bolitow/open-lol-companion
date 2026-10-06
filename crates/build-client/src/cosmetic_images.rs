//! Cache borné des images cosmétiques publiques versionnées (#93).

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CosmeticImageError {
    #[error("cosmetic_image_invalid_url")]
    InvalidUrl,
    #[error("cosmetic_image_unavailable")]
    Unavailable,
    #[error("cosmetic_image_invalid_response")]
    InvalidResponse,
    #[error("cosmetic_image_storage")]
    Storage,
}
#[derive(Debug, PartialEq, Eq)]
pub struct CosmeticImage {
    pub bytes: Vec<u8>,
    pub media_type: String,
}
const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 128 * 1024 * 1024;
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

fn validate_url(url: &str) -> Result<&'static str> {
    let invalid = CosmeticImageError::InvalidUrl;
    if url.len() > 2048 || !url.is_ascii() {
        return Err(invalid);
    }
    if let Some(path) = url.strip_prefix("https://ddragon.leagueoflegends.com/cdn/") {
        let (version, path) = path.split_once('/').ok_or(invalid)?;
        let id = path
            .strip_prefix("img/profileicon/")
            .and_then(|p| p.strip_suffix(".png"))
            .ok_or(invalid)?;
        if !version_valid(version, 3)
            || id.is_empty()
            || id.len() > 10
            || !id.bytes().all(|b| b.is_ascii_digit())
            || id.parse::<u32>().is_err()
        {
            return Err(invalid);
        }
        return Ok("image/png");
    }
    if let Some(path) = url.strip_prefix("https://raw.communitydragon.org/") {
        let (version, path) = path.split_once('/').ok_or(invalid)?;
        let path = path
            .strip_prefix("plugins/rcp-be-lol-game-data/global/default/assets/")
            .ok_or(invalid)?;
        if !version_valid(version, 2)
            || path.split('/').any(|p| {
                p.is_empty()
                    || p == "."
                    || p == ".."
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            })
        {
            return Err(invalid);
        }
        return match path.rsplit_once('.').map(|(_, extension)| extension) {
            Some("png") => Ok("image/png"),
            Some("jpg" | "jpeg") => Ok("image/jpeg"),
            Some("webp") => Ok("image/webp"),
            _ => Err(invalid),
        };
    }
    Err(invalid)
}
fn version_valid(version: &str, count: usize) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == count
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.bytes().all(|b| b.is_ascii_digit()))
}
fn validate_image(bytes: &[u8], media_type: &str) -> Result<()> {
    let valid = bytes.len() <= MAX_IMAGE_BYTES
        && match media_type {
            "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image/jpeg" => bytes.starts_with(b"\xff\xd8\xff"),
            "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
            _ => false,
        };
    if valid {
        Ok(())
    } else {
        Err(CosmeticImageError::InvalidResponse)
    }
}

use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    future::Future,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

type Result<T> = std::result::Result<T, CosmeticImageError>;
const RECORD_HEADER: usize = 41;
const RECORD_MAGIC: &[u8; 8] = b"OLCIMG01";
const MAX_ENTRIES: usize = 4096;

/// Un cache dédié aux images publiques : aucun identifiant de compte ni jeton.
/// Les chargements sont sérialisés, y compris entre processus partageant le dossier.
pub struct CosmeticImages {
    root: PathBuf,
    budget: u64,
    http: reqwest::Client,
    gate: tokio::sync::Mutex<()>,
}
impl CosmeticImages {
    pub fn new(root: PathBuf) -> Result<Self> {
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| CosmeticImageError::Unavailable)?
        .with_root_certificates(rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        })
        .with_no_client_auth();
        let http = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(TIMEOUT)
            .build()
            .map_err(|_| CosmeticImageError::Unavailable)?;
        Ok(Self {
            root,
            budget: MAX_CACHE_BYTES,
            http,
            gate: tokio::sync::Mutex::new(()),
        })
    }
    /// La file attend son tour ; chaque chargement actif dispose ensuite de 30 secondes.
    pub async fn load(&self, url: &str) -> Result<CosmeticImage> {
        self.load_with(url, || self.download(url)).await
    }
    async fn load_with<F, Fut>(&self, url: &str, fetch: F) -> Result<CosmeticImage>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<CosmeticImage>>,
    {
        self.load_with_timeout(url, fetch, TIMEOUT).await
    }
    async fn load_with_timeout<F, Fut>(
        &self,
        url: &str,
        fetch: F,
        timeout: Duration,
    ) -> Result<CosmeticImage>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<CosmeticImage>>,
    {
        let expected = validate_url(url)?;
        let key = format!("{:x}.image", Sha256::digest(url.as_bytes()));
        // Une grille peut demander plusieurs dizaines d'images : leur attente n'est pas une panne réseau.
        let _gate = self.gate.lock().await;
        tokio::time::timeout(timeout, async {
            let root = self.root.clone();
            let name = key.clone();
            let budget = self.budget;
            let (lease, cached) = blocking(move || {
                let lease = acquire(&root)?;
                let cached = read_cached(&root.join(name), expected)?;
                prune(&root, budget, 0, 0)?;
                Ok((lease, cached))
            })
            .await?;
            if let Some(image) = cached {
                return Ok(image);
            }
            let image = fetch().await?;
            if image.media_type != expected {
                return Err(CosmeticImageError::InvalidResponse);
            }
            validate_image(&image.bytes, expected)?;
            let root = self.root.clone();
            blocking(move || {
                // Le verrou reste détenu pendant le réseau puis l'écriture atomique.
                let _lease = lease;
                write_cached(&root, &key, &image, budget)?;
                Ok(image)
            })
            .await
        })
        .await
        .map_err(|_| CosmeticImageError::Unavailable)?
    }
    async fn download(&self, url: &str) -> Result<CosmeticImage> {
        let expected = validate_url(url)?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|_| CosmeticImageError::Unavailable)?;
        read_response(response, expected).await
    }
}
async fn read_response(mut response: reqwest::Response, expected: &str) -> Result<CosmeticImage> {
    if !response.status().is_success() {
        return Err(CosmeticImageError::Unavailable);
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.split(';').next())
        .map(str::trim);
    if mime != Some(expected)
        || response
            .content_length()
            .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        return Err(CosmeticImageError::InvalidResponse);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| CosmeticImageError::Unavailable)?
    {
        if chunk.len() > MAX_IMAGE_BYTES.saturating_sub(bytes.len()) {
            return Err(CosmeticImageError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    validate_image(&bytes, expected)?;
    Ok(CosmeticImage {
        bytes,
        media_type: expected.into(),
    })
}
async fn blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|_| CosmeticImageError::Unavailable)?
}
fn storage<T>(result: std::io::Result<T>) -> Result<T> {
    result.map_err(|_| CosmeticImageError::Storage)
}
fn acquire(root: &Path) -> Result<File> {
    storage(fs::create_dir_all(root))?;
    let meta = storage(fs::symlink_metadata(root))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(CosmeticImageError::Storage);
    }
    let path = root.join("images.lock");
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err(CosmeticImageError::Storage);
    }
    let file = storage(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path),
    )?;
    let start = Instant::now();
    loop {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(file),
            // fs2 renvoie ERROR_LOCK_VIOLATION sous Windows, pas nécessairement WouldBlock.
            Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                if start.elapsed() >= TIMEOUT {
                    return Err(CosmeticImageError::Unavailable);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return Err(CosmeticImageError::Storage),
        }
    }
}
fn read_cached(path: &Path, expected: &str) -> Result<Option<CosmeticImage>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(CosmeticImageError::Storage),
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(CosmeticImageError::Storage);
    }
    if meta.len() > (MAX_IMAGE_BYTES + RECORD_HEADER) as u64 {
        storage(fs::remove_file(path))?;
        return Ok(None);
    }
    let mut bytes = Vec::new();
    storage(File::open(path))?
        .take((MAX_IMAGE_BYTES + RECORD_HEADER + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CosmeticImageError::Storage)?;
    let code = match expected {
        "image/png" => 1,
        "image/jpeg" => 2,
        "image/webp" => 3,
        _ => 0,
    };
    let valid = bytes.len() > RECORD_HEADER
        && bytes.len() <= MAX_IMAGE_BYTES + RECORD_HEADER
        && bytes.starts_with(RECORD_MAGIC)
        && bytes[40] == code
        && bytes[8..40] == Sha256::digest(&bytes[RECORD_HEADER..])[..]
        && validate_image(&bytes[RECORD_HEADER..], expected).is_ok();
    if !valid {
        storage(fs::remove_file(path))?;
        return Ok(None);
    }
    // Windows exige FILE_WRITE_ATTRIBUTES ; read + write le fournit sans tronquer le contenu.
    let file = storage(OpenOptions::new().read(true).write(true).open(path))?;
    storage(file.set_modified(SystemTime::now()))?;
    Ok(Some(CosmeticImage {
        bytes: bytes.split_off(RECORD_HEADER),
        media_type: expected.into(),
    }))
}
fn managed_name(name: &str) -> bool {
    name.strip_suffix(".image").is_some_and(|stem| {
        stem.len() == 64
            && stem
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn prune(root: &Path, budget: u64, incoming: u64, incoming_count: usize) -> Result<()> {
    if incoming > budget {
        return Err(CosmeticImageError::Storage);
    }
    let mut entries = Vec::new();
    let mut total = incoming;
    for entry in storage(fs::read_dir(root))? {
        let entry = storage(entry)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Seuls nos fichiers temporaires abandonnés sont supprimés sous le verrou.
        if name.starts_with(".cosmetic-") && name.ends_with(".tmp") {
            if storage(entry.file_type())?.is_file() {
                storage(fs::remove_file(entry.path()))?;
            }
            continue;
        }
        if !managed_name(&name) {
            continue;
        }
        let meta = storage(fs::symlink_metadata(entry.path()))?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            continue;
        }
        total = total.saturating_add(meta.len());
        entries.push((
            meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            name.into_owned(),
            meta.len(),
        ));
    }
    entries.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let mut count = entries.len() + incoming_count;
    for (_, name, bytes) in entries {
        if total <= budget && count <= MAX_ENTRIES {
            break;
        }
        storage(fs::remove_file(root.join(name)))?;
        total = total.saturating_sub(bytes);
        count -= 1;
    }
    Ok(())
}
fn write_cached(root: &Path, key: &str, image: &CosmeticImage, budget: u64) -> Result<()> {
    let size = (image.bytes.len() + RECORD_HEADER) as u64;
    prune(root, budget, size, 1)?;
    let code = match image.media_type.as_str() {
        "image/png" => 1,
        "image/jpeg" => 2,
        "image/webp" => 3,
        _ => return Err(CosmeticImageError::InvalidResponse),
    };
    let mut temp = tempfile::Builder::new()
        .prefix(".cosmetic-")
        .suffix(".tmp")
        .tempfile_in(root)
        .map_err(|_| CosmeticImageError::Storage)?;
    storage(temp.write_all(RECORD_MAGIC))?;
    storage(temp.write_all(&Sha256::digest(&image.bytes)))?;
    storage(temp.write_all(&[code]))?;
    storage(temp.write_all(&image.bytes))?;
    storage(temp.as_file().sync_all())?;
    temp.persist(root.join(key))
        .map_err(|_| CosmeticImageError::Storage)?;
    #[cfg(unix)]
    storage(storage(File::open(root))?.sync_all())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const PROFILE: &str = "https://ddragon.leagueoflegends.com/cdn/16.19.1/img/profileicon/29.png";
    const SPLASH: &str = "https://raw.communitydragon.org/16.19/plugins/rcp-be-lol-game-data/global/default/assets/characters/ahri/skins/skin07/images/ahri_splash_centered_7.jpg";
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01";

    #[test]
    fn accepte_seulement_les_deux_cdn_versionnes_et_formats_prevus() {
        assert_eq!(validate_url(PROFILE), Ok("image/png"));
        assert_eq!(validate_url(SPLASH), Ok("image/jpeg"));
        assert_eq!(
            validate_url(&SPLASH.replace(".jpg", ".webp")),
            Ok("image/webp")
        );
        assert_eq!(
            validate_url(&SPLASH.replace(".jpg", ".jpeg")),
            Ok("image/jpeg")
        );
    }
    #[test]
    fn refuse_alias_traversal_query_identifiants_et_origines_non_autorises() {
        let invalid = [
            PROFILE.replace("https:", "http:"),
            PROFILE.replace("16.19.1", "latest"),
            PROFILE.replace("16.19.1", "16.19"),
            PROFILE.replace("29.png", "abc.png"),
            PROFILE.replace("29.png", "../29.png"),
            PROFILE.replace("29.png", "%32%39.png"),
            PROFILE.replace("29.png", "29.svg"),
            PROFILE.replace("/img/", "/image/"),
            PROFILE.replace(".com/", ".com:443/"),
            PROFILE.replace("https://", "https://user@"),
            format!("{PROFILE}?x=1"),
            format!("{PROFILE}#x"),
            format!(" {PROFILE}"),
            PROFILE.replace(".com/", ".com.evil/"),
            SPLASH.replace("16.19", "latest"),
            SPLASH.replace("/characters/", "/../"),
            SPLASH.replace("/characters/", "/%2e%2e/"),
            SPLASH.replace("/characters/", "/./"),
            SPLASH.replace("/characters/", "//"),
            SPLASH.replace("/characters/", "/..\\"),
            SPLASH.replace(".jpg", ".json"),
        ];
        for url in invalid {
            assert_eq!(
                validate_url(&url),
                Err(CosmeticImageError::InvalidUrl),
                "{url}"
            );
        }
    }
    #[test]
    fn valide_signature_et_type_sans_accepter_html_ou_image_vide() {
        assert_eq!(validate_image(PNG, "image/png"), Ok(()));
        assert_eq!(
            validate_image(b"\xff\xd8\xff\xe0jpeg", "image/jpeg"),
            Ok(())
        );
        assert_eq!(
            validate_image(b"RIFF\x10\0\0\0WEBPVP8 test", "image/webp"),
            Ok(())
        );
        for (bytes, mime) in [
            (PNG, "image/jpeg"),
            (b"<html>error</html>".as_slice(), "image/png"),
            (b"".as_slice(), "image/png"),
            (PNG, "image/svg+xml"),
        ] {
            assert_eq!(
                validate_image(bytes, mime),
                Err(CosmeticImageError::InvalidResponse)
            );
        }
    }
    fn png() -> CosmeticImage {
        CosmeticImage {
            bytes: PNG.to_vec(),
            media_type: "image/png".into(),
        }
    }
    fn path_for(root: &std::path::Path, url: &str) -> std::path::PathBuf {
        use sha2::{Digest, Sha256};
        root.join(format!("{:x}.image", Sha256::digest(url.as_bytes())))
    }
    #[tokio::test]
    async fn reutilise_le_cache_apres_redemarrage_sans_reseau() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        assert_eq!(
            cache
                .load_with(PROFILE, || async { Ok(png()) })
                .await
                .unwrap(),
            png()
        );
        let reopened = CosmeticImages::new(dir.path().into()).unwrap();
        assert_eq!(
            reopened
                .load_with(PROFILE, || async { Err(CosmeticImageError::Unavailable) })
                .await
                .unwrap(),
            png()
        );
    }
    #[tokio::test]
    async fn refuse_image_invalide_sans_polluer_le_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        assert_eq!(
            cache
                .load_with(PROFILE, || async {
                    Ok(CosmeticImage {
                        bytes: b"html".to_vec(),
                        media_type: "image/png".into(),
                    })
                })
                .await,
            Err(CosmeticImageError::InvalidResponse)
        );
        assert!(!path_for(dir.path(), PROFILE).exists());
    }
    #[tokio::test]
    async fn refuse_url_avant_fournisseur_et_acces_disque() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        let cache = CosmeticImages::new(root.clone()).unwrap();
        assert_eq!(
            cache
                .load_with("https://evil.test/x.png", || async {
                    panic!("réseau interdit")
                })
                .await,
            Err(CosmeticImageError::InvalidUrl)
        );
        assert!(!root.exists());
    }
    #[tokio::test]
    async fn detecte_corruption_et_retelecharge_sans_servir_les_octets_alteres() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        cache
            .load_with(PROFILE, || async { Ok(png()) })
            .await
            .unwrap();
        let path = path_for(dir.path(), PROFILE);
        let mut stored = std::fs::read(&path).unwrap();
        let end = stored.len() - 1;
        stored[end] ^= 1;
        std::fs::write(&path, stored).unwrap();
        assert_eq!(
            cache
                .load_with(PROFILE, || async { Err(CosmeticImageError::Unavailable) })
                .await,
            Err(CosmeticImageError::Unavailable)
        );
        assert!(!path.exists());
        assert_eq!(
            cache
                .load_with(PROFILE, || async { Ok(png()) })
                .await
                .unwrap(),
            png()
        );
    }
    #[tokio::test]
    async fn evince_le_moins_recemment_lu_sans_effacer_les_fichiers_etrangers() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = CosmeticImages::new(dir.path().into()).unwrap();
        cache.budget = 180;
        let second = PROFILE.replace("29.png", "30.png");
        let third = PROFILE.replace("29.png", "31.png");
        cache
            .load_with(PROFILE, || async { Ok(png()) })
            .await
            .unwrap();
        cache
            .load_with(&second, || async { Ok(png()) })
            .await
            .unwrap();
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path_for(dir.path(), &second))
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH)
            .unwrap();
        std::fs::write(dir.path().join("keep.txt"), b"user").unwrap();
        cache
            .load_with(PROFILE, || async { panic!("cache attendu") })
            .await
            .unwrap();
        cache
            .load_with(&third, || async { Ok(png()) })
            .await
            .unwrap();
        assert!(path_for(dir.path(), PROFILE).exists());
        assert!(!path_for(dir.path(), &second).exists());
        assert!(path_for(dir.path(), &third).exists());
        assert_eq!(std::fs::read(dir.path().join("keep.txt")).unwrap(), b"user");
    }
    #[tokio::test]
    async fn deduplique_les_appels_de_deux_instances_du_meme_cache() {
        let dir = tempfile::tempdir().unwrap();
        let a = CosmeticImages::new(dir.path().into()).unwrap();
        let b = CosmeticImages::new(dir.path().into()).unwrap();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let fetch = || async {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok(png())
        };
        let (first, second) =
            tokio::join!(a.load_with(PROFILE, fetch), b.load_with(PROFILE, fetch));
        assert_eq!(first.unwrap(), png());
        assert_eq!(second.unwrap(), png());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    async fn response(cache: &CosmeticImages, headers: &str, body: Vec<u8>) -> reqwest::Response {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let headers = headers.to_owned();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request).await;
            let _ = stream.write_all(headers.as_bytes()).await;
            let _ = stream.write_all(&body).await;
        });
        cache
            .http
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn verifie_mime_signature_et_redirection_de_la_reponse_http() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        let headers=format!("HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",PNG.len());
        assert_eq!(
            read_response(response(&cache, &headers, PNG.to_vec()).await, "image/png")
                .await
                .unwrap(),
            png()
        );
        let headers = headers.replace("image/png", "text/html");
        assert_eq!(
            read_response(response(&cache, &headers, PNG.to_vec()).await, "image/png").await,
            Err(CosmeticImageError::InvalidResponse)
        );
        let redirect="HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(
            read_response(response(&cache, redirect, vec![]).await, "image/png").await,
            Err(CosmeticImageError::Unavailable)
        );
    }
    #[tokio::test]
    async fn refuse_depassement_annonce_et_chunked_sans_content_length() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        let headers="HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 8388609\r\nConnection: close\r\n\r\n";
        assert_eq!(
            read_response(response(&cache, headers, vec![]).await, "image/png").await,
            Err(CosmeticImageError::InvalidResponse)
        );
        let headers="HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        let mut body = b"800001\r\n".to_vec();
        body.extend_from_slice(PNG);
        body.resize(8 + MAX_IMAGE_BYTES + 1, 0);
        body.extend_from_slice(b"\r\n0\r\n\r\n");
        assert_eq!(
            read_response(response(&cache, headers, body).await, "image/png").await,
            Err(CosmeticImageError::InvalidResponse)
        );
    }
    #[tokio::test]
    async fn annuler_un_chargement_libere_le_verrou_et_ne_publie_rien() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        let cancelled = tokio::time::timeout(
            Duration::from_millis(30),
            cache.load_with(PROFILE, || async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(png())
            }),
        )
        .await;
        assert!(cancelled.is_err());
        assert!(!path_for(dir.path(), PROFILE).exists());
        let image = tokio::time::timeout(
            Duration::from_secs(1),
            cache.load_with(PROFILE, || async { Ok(png()) }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(image, png());
    }
    #[tokio::test]
    async fn attente_dans_la_file_ne_consomme_pas_le_delai_de_chaque_image() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CosmeticImages::new(dir.path().into()).unwrap();
        use std::{future::Future, task::Poll};
        // Bloquer la file explicitement, puis dépasser le délai réseau en temps virtuel.
        // Le disque n'est sollicité qu'après reprise du temps réel, avec sa marge normale.
        let gate = cache.gate.lock().await;
        tokio::time::pause();
        let mut pending = std::pin::pin!(cache.load_with_timeout(
            PROFILE,
            || async { Ok(png()) },
            Duration::from_secs(20),
        ));
        std::future::poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        tokio::time::advance(Duration::from_secs(21)).await;
        std::future::poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        tokio::time::resume();
        drop(gate);
        assert_eq!(pending.await.unwrap(), png());
    }
}
