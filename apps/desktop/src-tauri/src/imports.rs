use lcu_connector::imports::{ImportError, ImportRunesRequest};
use lcu_connector::LcuClient;
use tauri::async_runtime::{spawn_blocking, Mutex};
use tauri::State;

/// Sérialise la lecture puis l'écriture pour chaque type d'import.
#[derive(Default)]
pub(crate) struct ImportLocks {
    runes: Mutex<()>,
    spells: Mutex<()>,
    items: Mutex<()>,
    auto_history: std::sync::Mutex<
        std::collections::VecDeque<(
            lcu_connector::imports::AutoImportContext,
            bool,
            lcu_connector::imports::AutoImportReceipt,
        )>,
    >,
}

/// L'historique de cette exécution survit aux remontages du webview.
#[tauri::command]
pub(crate) async fn import_selected_build(
    request: lcu_connector::imports::AutoImportRequest,
    session: State<'_, crate::SessionState>,
    locks: State<'_, ImportLocks>,
) -> Result<lcu_connector::imports::AutoImportReceipt, lcu_connector::imports::DraftRuneImportError>
{
    use lcu_connector::imports::{AutoImportSelection, DraftRuneGuardError, DraftRuneImportError};
    let current_draft = || {
        session.lock().is_ok_and(|state| {
            state.connected
                && state.phase == Some(lcu_connector::GameflowPhase::ChampSelect)
                && state.draft.is_some()
                && state.draft_id.as_deref() == Some(request.context.draft_id.as_str())
        })
    };
    if !current_draft() {
        return Err(DraftRuneImportError::Guard(
            DraftRuneGuardError::DraftContextChanged,
        ));
    }
    let runes = matches!(&request.selection, AutoImportSelection::Runes(_));
    let lock = if runes { &locks.runes } else { &locks.items };
    let _guard = lock
        .try_lock()
        .map_err(|_| DraftRuneImportError::Guard(DraftRuneGuardError::ImportBusy))?;
    {
        let history = locks
            .auto_history
            .lock()
            .map_err(|_| ImportError::ClientUnavailable)?;
        if let Some((_, _, receipt)) = history
            .iter()
            .find(|(context, kind, _)| context.same_selection(&request.context) && *kind == runes)
        {
            return Ok(*receipt);
        }
    }
    let receipt = import_client()
        .await?
        .import_selected_build(&request, current_draft)
        .await?;
    let mut history = locks
        .auto_history
        .lock()
        .map_err(|_| ImportError::ClientUnavailable)?;
    // Un retour au champion précédent après échange doit réappliquer son build.
    history.retain(|(_, kind, _)| *kind != runes);
    history.push_back((request.context, runes, receipt));
    Ok(receipt)
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

/// Import explicite du set consulté ; aucun besoin de modifier la sélection du champion.
#[tauri::command]
pub(crate) async fn import_items(
    request: lcu_connector::imports::ImportItemsRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), ImportError> {
    let _guard = locks.items.lock().await;
    import_client().await?.import_items(&request).await
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

/// Le front #13 exige une draft courante ; les clics concurrents ne sont pas mis en attente.
#[tauri::command]
pub(crate) async fn import_draft_runes(
    request: lcu_connector::imports::ImportDraftRunesRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), lcu_connector::imports::DraftRuneImportError> {
    use lcu_connector::imports::{DraftRuneGuardError, DraftRuneImportError};
    let _guard = locks
        .runes
        .try_lock()
        .map_err(|_| DraftRuneImportError::Guard(DraftRuneGuardError::ImportBusy))?;
    import_client().await?.import_draft_runes(&request).await
}

/// Contrat générique #15 de Matthieu ; le front emploie la garde de draft ci-dessous.
#[tauri::command]
pub(crate) async fn import_spells(
    request: lcu_connector::imports::ImportSpellsRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), ImportError> {
    let _guard = locks.spells.lock().await;
    import_client().await?.import_spells(&request).await
}

#[tauri::command]
pub(crate) async fn import_draft_spells(
    request: lcu_connector::imports::ImportDraftSpellsRequest,
    locks: State<'_, ImportLocks>,
) -> Result<(), lcu_connector::imports::DraftRuneImportError> {
    use lcu_connector::imports::{DraftRuneGuardError, DraftRuneImportError};
    let _guard = locks
        .spells
        .try_lock()
        .map_err(|_| DraftRuneImportError::Guard(DraftRuneGuardError::ImportBusy))?;
    import_client().await?.import_draft_spells(&request).await
}
