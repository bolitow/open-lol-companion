//! Lecteur SkinSpotlights isolé, sans permission IPC pour le contenu distant.

pub(super) fn video_urls(
    video_id: &str,
    start: Option<u32>,
    end: Option<u32>,
) -> Result<(String, String), String> {
    if video_id.len() != 11
        || !video_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("spotlight_invalid_video".into());
    }
    if start.is_some_and(|value| value > 7200)
        || end.is_some_and(|value| value > 7200 || start.map_or(true, |start| value <= start))
    {
        return Err("spotlight_invalid_segment".into());
    }
    let mut embed = format!("https://www.youtube.com/embed/{video_id}?autoplay=0&rel=0");
    let mut external = format!("https://www.youtube.com/watch?v={video_id}");
    if let Some(start) = start {
        embed.push_str(&format!("&start={start}"));
        external.push_str(&format!("&t={start}s"));
    }
    if let Some(end) = end {
        embed.push_str(&format!("&end={end}"));
    }
    Ok((embed, external))
}

pub(super) fn navigation_allowed(url: &str) -> bool {
    if url == "about:blank" {
        return true;
    }
    let Some(path) = url.strip_prefix("https://www.youtube.com/embed/") else {
        return false;
    };
    let Some((video, query)) = path.split_once('?') else {
        return false;
    };
    let Some(extra) = query.strip_prefix("autoplay=0&rel=0") else {
        return false;
    };
    let mut start = None;
    let mut end = None;
    if !extra.is_empty() {
        let Some(bounds) = extra.strip_prefix("&start=") else {
            return false;
        };
        let (from, to) = bounds
            .split_once("&end=")
            .map_or((bounds, None), |(a, b)| (a, Some(b)));
        start = from.parse::<u32>().ok();
        if start.is_none() {
            return false;
        }
        if let Some(to) = to {
            end = to.parse::<u32>().ok();
            if end.is_none() {
                return false;
            }
        }
    }
    video_urls(video, start, end).is_ok_and(|(expected, _)| expected == url)
}

fn caller_allowed(label: &str) -> bool {
    label == "main"
}

#[tauri::command]
pub async fn open_skin_spotlight(
    app: tauri::AppHandle,
    window: tauri::Webview,
    video_id: String,
    external: bool,
    start_seconds: Option<u32>,
    end_seconds: Option<u32>,
) -> Result<(), String> {
    use tauri::Manager;

    if !caller_allowed(window.label()) {
        return Err("spotlight_forbidden".into());
    }
    let (embed_url, external_url) = video_urls(&video_id, start_seconds, end_seconds)?;
    if external {
        return tauri::async_runtime::spawn_blocking(move || open_external(&external_url))
            .await
            .map_err(|_| "spotlight_external_failed".to_string())?;
    }

    let existing = app.get_webview_window("skin-spotlight");
    let created = existing.is_none();
    let player = match existing {
        Some(player) => player,
        None => tauri::WebviewWindowBuilder::new(
            &app,
            "skin-spotlight",
            tauri::WebviewUrl::External(
                "about:blank".parse().map_err(|_| "spotlight_unavailable")?,
            ),
        )
        .title("SkinSpotlights")
        .inner_size(960.0, 540.0)
        .min_inner_size(480.0, 270.0)
        .center()
        .visible(false)
        .incognito(true)
        .on_navigation(|url| navigation_allowed(url.as_str()))
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build()
        .map_err(|_| "spotlight_unavailable".to_string())?,
    };
    // L'identité installée provient de la configuration, jamais du contenu distant.
    let referer = format!("https://{}/", app.config().identifier.to_ascii_lowercase());
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let scheduled = player.with_webview(move |platform| {
        let _ = sender.send(navigate_with_referer(platform, &embed_url, &referer));
    });
    let result = match scheduled {
        Ok(()) => receiver
            .await
            .unwrap_or_else(|_| Err("spotlight_unavailable".into())),
        Err(_) => Err("spotlight_unavailable".into()),
    };
    if let Err(error) = result {
        if created {
            let _ = player.close();
        }
        return Err(error);
    }
    player
        .show()
        .map_err(|_| "spotlight_unavailable".to_string())?;
    player
        .set_focus()
        .map_err(|_| "spotlight_unavailable".to_string())
}

#[cfg(target_os = "macos")]
pub(super) fn navigate_with_referer(
    platform: tauri::webview::PlatformWebview,
    embed_url: &str,
    referer: &str,
) -> Result<(), String> {
    use objc2_foundation::{NSMutableURLRequest, NSString, NSURL};
    use objc2_web_kit::WKWebView;

    objc2::rc::autoreleasepool(|_| {
        let url = NSURL::URLWithString(&NSString::from_str(embed_url))
            .ok_or_else(|| "spotlight_invalid_video".to_string())?;
        let request = NSMutableURLRequest::requestWithURL(&url);
        request.setValue_forHTTPHeaderField(
            Some(&NSString::from_str(referer)),
            &NSString::from_str("Referer"),
        );
        // Tauri garantit le handle WKWebView et l'exécution sur le thread principal.
        unsafe {
            let view: &WKWebView = &*platform.inner().cast();
            view.loadRequest(&request)
                .map(|_| ())
                .ok_or_else(|| "spotlight_unavailable".to_string())
        }
    })
}

#[cfg(target_os = "windows")]
pub(super) fn navigate_with_referer(
    platform: tauri::webview::PlatformWebview,
    embed_url: &str,
    referer: &str,
) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment9, ICoreWebView2_10,
    };
    use windows_core::{Interface, HSTRING};

    // Même adaptation que Wry 0.57 : les headers font partie de la requête native.
    let navigate = || -> windows_core::Result<()> {
        unsafe {
            let environment = platform.environment().cast::<ICoreWebView2Environment9>()?;
            let view = platform
                .controller()
                .CoreWebView2()?
                .cast::<ICoreWebView2_10>()?;
            let request = environment.CreateWebResourceRequest(
                &HSTRING::from(embed_url),
                &HSTRING::from("GET"),
                None,
                &HSTRING::from(format!("Referer: {referer}\r\n")),
            )?;
            view.NavigateWithWebResourceRequest(&request)
        }
    };
    navigate().map_err(|_| "spotlight_unavailable".to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn navigate_with_referer(
    _platform: tauri::webview::PlatformWebview,
    _embed_url: &str,
    _referer: &str,
) -> Result<(), String> {
    Err("spotlight_unavailable".into())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn open_external(url: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};

    // URL construite uniquement après validation stricte de l'identifiant, sans shell.
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("/usr/bin/open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", url]);
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW.
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "spotlight_external_failed".to_string())?
        .success()
        .then_some(())
        .ok_or_else(|| "spotlight_external_failed".to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn open_external(_url: &str) -> Result<(), String> {
    Err("spotlight_external_failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepte_uniquement_les_identifiants_youtube_ascii_de_onze_caracteres() {
        assert!(video_urls("Abc_123-xyZ", None, None).is_ok());
        for invalid in [
            "",
            "Abc1234567",
            "Abc123456789",
            "Abc1234567é",
            "Abc1234567/",
            "Abc1234567?",
            "Abc1234567\n",
            "../watch?v=",
            "https://youtube.com/watch?v=Abc_123-xyZ",
        ] {
            assert!(video_urls(invalid, None, None).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn construit_des_destinations_fixes_sans_lecture_automatique() {
        let (embed, external) = video_urls("Abc_123-xyZ", None, None).unwrap();
        assert_eq!(
            embed,
            "https://www.youtube.com/embed/Abc_123-xyZ?autoplay=0&rel=0"
        );
        assert_eq!(external, "https://www.youtube.com/watch?v=Abc_123-xyZ");
    }

    #[test]
    fn limite_la_navigation_au_lecteur_youtube_et_a_la_page_vide() {
        assert!(navigation_allowed("about:blank"));
        assert!(navigation_allowed(
            "https://www.youtube.com/embed/Abc_123-xyZ?autoplay=0&rel=0"
        ));
        for invalid in [
            "http://www.youtube.com/embed/Abc_123-xyZ?autoplay=0&rel=0",
            "https://www.youtube.com.evil.test/embed/Abc_123-xyZ?autoplay=0&rel=0",
            "https://www.youtube.com@evil.test/embed/Abc_123-xyZ?autoplay=0&rel=0",
            "https://www.youtube.com:444/embed/Abc_123-xyZ?autoplay=0&rel=0",
            "https://www.youtube.com/watch?v=Abc_123-xyZ",
            "https://www.youtube.com/embed/../watch?v=Abc_123-xyZ",
            "https://www.youtube.com/embed/Abc_123-xyZ?autoplay=1",
            "file:///etc/passwd",
            "tauri://localhost",
            "javascript:alert(1)",
            "about:blank#x",
        ] {
            assert!(!navigation_allowed(invalid), "{invalid}");
        }
    }

    #[test]
    fn borne_les_passages_et_conserve_le_repli_au_meme_instant() {
        let (embed, external) = video_urls("Abc_123-xyZ", Some(88), Some(97)).unwrap();
        assert!(embed.ends_with("&start=88&end=97"));
        assert!(external.ends_with("&t=88s"));
        assert!(navigation_allowed(&embed));
        for (start, end) in [
            (None, Some(10)),
            (Some(20), Some(20)),
            (Some(50), Some(10)),
            (Some(7201), None),
            (Some(1), Some(7201)),
        ] {
            assert!(video_urls("Abc_123-xyZ", start, end).is_err());
        }
    }

    #[test]
    fn refuse_les_commandes_hors_de_la_webview_principale() {
        assert!(caller_allowed("main"));
        for label in ["skin-spotlight", "overlay", "", "main-child"] {
            assert!(!caller_allowed(label));
        }
    }
}
