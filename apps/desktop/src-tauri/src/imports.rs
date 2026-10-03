mod guard;
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
    auto_history: std::sync::Mutex<guard::History>,
}

impl ImportLocks {
    pub(crate) fn set_custom_role(&self, role: Option<String>) -> Result<(), &'static str> {
        self.auto_history
            .lock()
            .map(|mut h| h.set_custom_role(role))
            .map_err(|_| "unavailable")
    }
    pub(crate) fn observe(&self, session: &lcu_connector::LcuSession) {
        if let Ok(mut history) = self.auto_history.lock() {
            history.observe(session);
        }
    }
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
    let runes = matches!(&request.selection, AutoImportSelection::Runes(_));
    let lock = if runes { &locks.runes } else { &locks.items };
    let _guard = lock
        .try_lock()
        .map_err(|_| DraftRuneImportError::Guard(DraftRuneGuardError::ImportBusy))?;
    let epoch =
        {
            // Même ordre que le producteur LCU : session, puis historique.
            let state = session.lock().map_err(|_| ImportError::ClientUnavailable)?;
            let mut history = locks
                .auto_history
                .lock()
                .map_err(|_| ImportError::ClientUnavailable)?;
            history.observe(&state);
            if !guard::matches_selection(&state, &request.context, history.custom_role.as_deref()) {
                return Err(DraftRuneImportError::Guard(
                    DraftRuneGuardError::DraftContextChanged,
                ));
            }
            if let Some((_, _, receipt)) = history.receipts.iter().find(|(context, kind, _)| {
                context.same_selection(&request.context) && *kind == runes
            }) {
                return Ok(*receipt);
            }
            history.epoch
        };
    let current_draft = || {
        session.lock().is_ok_and(|state| {
            locks.auto_history.lock().is_ok_and(|history| {
                history.epoch == epoch
                    && guard::matches_selection(
                        &state,
                        &request.context,
                        history.custom_role.as_deref(),
                    )
            })
        })
    };
    let receipt = import_client()
        .await?
        .import_selected_build(&request, current_draft)
        .await?;
    let state = session.lock().map_err(|_| ImportError::ClientUnavailable)?;
    let mut history = locks
        .auto_history
        .lock()
        .map_err(|_| ImportError::ClientUnavailable)?;
    // Ne jamais réinsérer un accusé tardif, y compris après A → B → A pendant la requête.
    if history.epoch == epoch
        && guard::matches_selection(&state, &request.context, history.custom_role.as_deref())
    {
        history.receipts.retain(|(_, kind, _)| *kind != runes);
        history.receipts.push((request.context, runes, receipt));
    }
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
