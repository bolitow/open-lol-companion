//! Lecture indépendante du client installé et du manifeste de données statiques.
use serde::Serialize;
use tauri::Manager;

/// Miroir exact de `BuildPatchContext` dans @olc/shared ; aucun secret transporté.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPatchContext {
    client: Option<lcu_connector::ClientPatch>,
    client_error: Option<lcu_connector::ClientPatchError>,
    manifest: Option<olc_build_client::static_versions::StaticVersions>,
    manifest_error: Option<olc_build_client::BuildError>,
}
#[tauri::command]
pub async fn build_patch_context(app: tauri::AppHandle) -> BuildPatchContext {
    let (client, manifest) = tokio::join!(super::client_patch(), async {
        let client = app.state::<crate::api_access::ApiState>().client().await?;
        let result = client.static_versions().await;
        if matches!(result, Err(olc_build_client::BuildError::Unauthorized)) {
            crate::api_access::report_rejection(&app, &client);
        }
        result
    });
    BuildPatchContext {
        client_error: client.as_ref().err().copied(),
        client: client.ok(),
        manifest_error: manifest.as_ref().err().copied(),
        manifest: manifest.ok(),
    }
}
