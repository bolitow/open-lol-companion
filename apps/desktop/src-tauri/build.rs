fn main() {
    // La fenêtre passive ne reçoit que les commandes de lecture explicitement autorisées.
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "desktop_settings",
            "set_desktop_setting",
            "set_desktop_locale",
            "export_diagnostics",
            "lcu_status",
            "lcu_session",
            "client_patch",
            "build_patch_context",
            "catalog_state",
            "catalog_sync",
            "catalog_read",
            "friends_state",
            "community_builds",
            "publication_state",
            "api_access_status",
            "save_api_access",
            "clear_api_access",
            "player_profile",
            "player_matches",
            "import_runes",
            "import_draft_runes",
            "import_spells",
            "import_draft_spells",
            "import_items",
            "import_selected_build",
            "live_session",
            "live_custom_role",
            "overlay_state",
            "overlay_content_height",
            "overlay_locale",
            "overlay_configure",
            "overlay_preview",
        ]));
    tauri_build::try_build(attributes).expect("configuration Tauri invalide");
}
