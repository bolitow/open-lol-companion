use lcu_connector::imports::{
    ImportError, ImportItemsRequest, ImportRunesRequest, ImportSpellsRequest,
};
use lcu_connector::LcuClient;
use tauri::async_runtime::{spawn_blocking, Mutex};
use tauri::State;

/// Sérialise la lecture puis l'écriture pour chaque type d'import.
#[derive(Default)]
pub(crate) struct ImportLocks {
    runes: Mutex<()>,
    spells: Mutex<()>,
    items: Mutex<()>,
}

async fn import_client() -> Result<LcuClient, ImportError> {
    // La découverte lit le disque et peut lancer un processus : hors du thread async.
    spawn_blocking(|| {
        let credentials = lcu_connector::discover().map_err(|_| ImportError::ClientUnavailable)?;
        LcuClient::new(&credentials).map_err(ImportError::from)
    })
    .await
    .map_err(|_| ImportError::ClientUnavailable)?
}

/// Appel explicite de l'interface après action du joueur ou activation de son réglage.
#[tauri::command]
pub(crate) async fn import_runes(
    request: ImportRunesRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), ImportError> {
    let _guard = locks.runes.lock().await;
    import_client().await?.import_runes(&request).await
}

#[tauri::command]
pub(crate) async fn import_spells(
    request: ImportSpellsRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), ImportError> {
    let _guard = locks.spells.lock().await;
    import_client().await?.import_spells(&request).await
}

#[tauri::command]
pub(crate) async fn import_items(
    request: ImportItemsRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), ImportError> {
    let _guard = locks.items.lock().await;
    import_client().await?.import_items(&request).await
}
