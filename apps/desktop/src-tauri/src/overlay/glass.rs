/// Installe une seule fois le matériau natif, sur le thread principal, avant affichage.
/// Le résultat distingue Liquid Glass (true) de la vibrance des anciens macOS (false).
#[cfg(target_os = "macos")]
pub fn install(
    window: &tauri::WebviewWindow,
    configure_panel: impl FnOnce() -> Result<(), ()>,
) -> Result<bool, ()> {
    // Une exception AppKit ne doit jamais traverser le callback C de lancement Tauri.
    objc2::exception::catch(std::panic::AssertUnwindSafe(|| {
        install_inner(window, configure_panel)
    }))
    .map_err(|_| ())?
}
#[cfg(target_os = "macos")]
fn install_inner(
    window: &tauri::WebviewWindow,
    configure_panel: impl FnOnce() -> Result<(), ()>,
) -> Result<bool, ()> {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua,
        NSAutoresizingMaskOptions, NSGlassEffectView, NSGlassEffectViewStyle, NSView,
        NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
        NSVisualEffectView, NSWindow,
    };

    let mtm = MainThreadMarker::new().ok_or(())?;
    let pointer = window.ns_window().map_err(|_| ())?;
    // Tauri garde la fenêtre vivante ; NSPanel hérite de NSWindow. L'accès reste sur le thread AppKit.
    let native_window = unsafe { pointer.cast::<NSWindow>().as_ref() }.ok_or(())?;
    let root = native_window.contentView().ok_or(())?;
    let bounds = root.bounds();
    let autoresize =
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable;
    // Retenir toutes les vues avant de changer leurs parents évite de muter l'énumération AppKit.
    let children: Vec<_> = root.subviews().iter().collect();
    if children.is_empty() {
        return Err(());
    }
    // Constante AppKit disponible depuis macOS 10.14, avant la cible minimale du projet (13).
    let appearance = NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }).ok_or(())?;

    // WebKit observe la classe dynamique de sa fenêtre par KVO. Le changement de
    // classe NSWindow → NSPanel doit se faire sans WebView attachée : elle retire
    // ses observateurs de l'ancienne classe puis les réinscrit sur la classe finale.
    // La racine Wry et les enfants retenus restent vivants pendant cette transition.
    for child in &children {
        child.removeFromSuperview();
    }
    configure_panel()?;

    if objc2::available!(macos = 26.0) {
        let glass = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), bounds);
        glass.setAutoresizingMask(autoresize);
        glass.setAppearance(Some(&appearance));
        glass.setStyle(NSGlassEffectViewStyle::Regular);
        glass.setCornerRadius(12.0);

        let content = NSView::initWithFrame(NSView::alloc(mtm), bounds);
        content.setAutoresizingMask(autoresize);
        for child in children {
            content.addSubview(&child);
        }
        // Apple garantit la composition du verre pour contentView, pas pour des sous-vues arbitraires.
        glass.setContentView(Some(&content));
        // Conserver WryWebViewParent comme racine préserve les références détenues par Wry/Tauri.
        root.addSubview(&glass);
        Ok(true)
    } else {
        let vibrancy = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), bounds);
        vibrancy.setAutoresizingMask(autoresize);
        vibrancy.setAppearance(Some(&appearance));
        vibrancy.setMaterial(NSVisualEffectMaterial::HUDWindow);
        vibrancy.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        // Un panneau non focalisable n'est jamais actif au sens d'AppKit.
        vibrancy.setState(NSVisualEffectState::Active);
        vibrancy.setWantsLayer(true);
        let layer = vibrancy.layer().ok_or(())?;
        layer.setCornerRadius(12.0);
        layer.setMasksToBounds(true);
        for child in children {
            vibrancy.addSubview(&child);
        }
        root.addSubview(&vibrancy);
        Ok(false)
    }
}
