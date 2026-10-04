//! Collection locale et souhaits par compte, sans identifiants techniques dans l'interface.
use lcu_connector::{
    collection::{read_collection, CollectionSkin, CollectionSnapshot},
    LcuAccount, LcuSession,
};
use olc_desktop_support::collection_wishes::{load_wishes, save_wishes};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
use tokio::sync::watch;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Disconnected,
    Loading,
    Ready,
    Unavailable,
}
#[derive(Clone, Serialize)]
pub struct CollectionState {
    pub revision: u64,
    pub status: Status,
    pub account: Option<LcuAccount>,
    pub skins: Vec<CollectionSkin>,
    pub wishes: Vec<u32>,
    pub storage_error: bool,
    pub stale: bool,
}
#[derive(Clone, Default, PartialEq, Eq)]
struct Input {
    generation: u64,
    account: Option<LcuAccount>,
}
pub(super) struct Runtime {
    input: Input,
    public: CollectionState,
    account_key: Option<String>,
    directory: Option<PathBuf>,
}
impl Runtime {
    fn new(directory: Option<PathBuf>) -> Self {
        Self {
            input: Input::default(),
            public: CollectionState {
                revision: 0,
                status: Status::Disconnected,
                account: None,
                skins: vec![],
                wishes: vec![],
                storage_error: directory.is_none(),
                stale: false,
            },
            account_key: None,
            directory,
        }
    }
    fn changed(&mut self, account: Option<LcuAccount>) -> bool {
        if self.input.account == account {
            return false;
        }
        self.input.generation = self.input.generation.saturating_add(1);
        self.input.account = account.clone();
        self.account_key = None;
        self.public.revision = self.public.revision.saturating_add(1);
        if account.is_some() {
            self.public.account = account;
            self.public.skins.clear();
            self.public.wishes.clear();
            self.public.storage_error = self.directory.is_none();
            self.public.status = Status::Loading;
            self.public.stale = false;
        } else {
            self.public.status = Status::Disconnected;
            self.public.stale = self.public.account.is_some();
        }
        true
    }
    fn refresh(&mut self) -> bool {
        if self.input.account.is_none() {
            return false;
        }
        self.input.generation = self.input.generation.saturating_add(1);
        self.public.revision = self.public.revision.saturating_add(1);
        self.public.status = Status::Loading;
        self.public.stale = !self.public.skins.is_empty();
        true
    }
    fn accept(
        &mut self,
        input: &Input,
        result: Option<(CollectionSnapshot, Result<BTreeSet<u32>, ()>)>,
    ) -> bool {
        if &self.input != input {
            return false;
        }
        self.public.revision = self.public.revision.saturating_add(1);
        if let Some((snapshot, wishes)) = result {
            self.account_key = Some(snapshot.account_key);
            self.public.skins = snapshot.skins;
            self.public.storage_error = wishes.is_err();
            self.public.wishes = wishes.unwrap_or_default().into_iter().collect();
            self.public.status = Status::Ready;
            self.public.stale = false;
        } else {
            self.account_key = None;
            self.public.status = Status::Unavailable;
            self.public.stale = !self.public.skins.is_empty();
        }
        true
    }
    fn wish(
        &mut self,
        revision: u64,
        id: u32,
        wished: bool,
    ) -> Result<CollectionState, &'static str> {
        if revision != self.public.revision {
            return Err("stale");
        }
        if self.public.status != Status::Ready || self.public.stale || self.input.account.is_none()
        {
            return Err("unavailable");
        }
        if !self.public.skins.iter().any(|skin| skin.id == id) {
            return Err("invalid_skin");
        }
        let path = self
            .directory
            .as_ref()
            .zip(self.account_key.as_ref())
            .map(|(dir, key)| dir.join(format!("{key}.json")))
            .ok_or("storage")?;
        let mut wishes = load_wishes(&path).map_err(|_| "storage")?;
        if wished {
            wishes.insert(id);
        } else {
            wishes.remove(&id);
        }
        save_wishes(&path, &wishes).map_err(|_| "storage")?;
        self.public.wishes = wishes.into_iter().collect();
        self.public.revision = self.public.revision.saturating_add(1);
        self.public.storage_error = false;
        Ok(self.public.clone())
    }
}
type Shared = Arc<Mutex<Runtime>>;
struct Control(watch::Sender<Input>);
#[tauri::command]
pub fn collection_state(state: tauri::State<'_, Shared>) -> Result<CollectionState, &'static str> {
    state
        .lock()
        .map(|s| s.public.clone())
        .map_err(|_| "unavailable")
}
#[tauri::command]
pub fn collection_refresh(app: tauri::AppHandle) -> Result<(), &'static str> {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().map_err(|_| "unavailable")?;
    if !state.refresh() {
        return Err("unavailable");
    }
    app.state::<Control>().0.send_replace(state.input.clone());
    let _ = app.emit_to("main", "collection-state", &state.public);
    Ok(())
}
#[tauri::command]
pub async fn collection_set_wish(
    app: tauri::AppHandle,
    revision: u64,
    skin_id: u32,
    wished: bool,
) -> Result<CollectionState, &'static str> {
    let shared = app.state::<Shared>().inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        shared
            .lock()
            .map_err(|_| "unavailable")?
            .wish(revision, skin_id, wished)
    })
    .await
    .map_err(|_| "unavailable")??;
    let _ = app.emit_to("main", "collection-state", &result);
    Ok(result)
}
pub fn lcu_changed(app: &tauri::AppHandle, session: &LcuSession) {
    let shared = app.state::<Shared>();
    if let Ok(mut state) = shared.lock() {
        if state.changed(session.account.clone().filter(|_| session.connected)) {
            app.state::<Control>().0.send_replace(state.input.clone());
            let _ = app.emit_to("main", "collection-state", &state.public);
        }
    };
}
async fn read(
    account: &LcuAccount,
    directory: Option<PathBuf>,
) -> Option<(CollectionSnapshot, Result<BTreeSet<u32>, ()>)> {
    let client = tauri::async_runtime::spawn_blocking(|| {
        lcu_connector::discover()
            .ok()
            .and_then(|credentials| lcu_connector::LcuClient::new(&credentials).ok())
    })
    .await
    .ok()
    .flatten()?;
    let snapshot = read_collection(&client, account).await.ok()?;
    let key = snapshot.account_key.clone();
    let wishes = tauri::async_runtime::spawn_blocking(move || {
        directory
            .ok_or(())
            .and_then(|dir| load_wishes(&dir.join(format!("{key}.json"))).map_err(|_| ()))
    })
    .await
    .unwrap_or(Err(()));
    Some((snapshot, wishes))
}
pub fn setup(app: &tauri::AppHandle) {
    let directory = app
        .path()
        .app_config_dir()
        .ok()
        .map(|p| p.join("collection-wishes"));
    let shared = Arc::new(Mutex::new(Runtime::new(directory.clone())));
    let (sender, mut input) = watch::channel(Input::default());
    app.manage(shared.clone());
    app.manage(Control(sender));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let current = input.borrow_and_update().clone();
            if let Some(account) = &current.account {
                let result = tokio::select! {biased; changed=input.changed()=>{if changed.is_err(){break;}continue;},result=read(account,directory.clone())=>result};
                if let Ok(mut state) = shared.lock() {
                    if state.accept(&current, result) {
                        let _ = app.emit_to("main", "collection-state", &state.public);
                    }
                }
            }
            if input.changed().await.is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn account(name: &str) -> LcuAccount {
        LcuAccount {
            platform: "EUW1".into(),
            game_name: name.into(),
            tag_line: "TEST".into(),
            profile_icon_id: None,
        }
    }
    fn ready(state: &mut Runtime, key: &str) {
        let input = state.input.clone();
        assert!(state.accept(
            &input,
            Some((
                CollectionSnapshot {
                    account_key: key.into(),
                    skins: vec![]
                },
                Ok(BTreeSet::from([103001]))
            ))
        ));
    }
    #[test]
    fn changement_compte_invalide_meme_une_reponse_apres_retour_sur_a() {
        let mut state = Runtime::new(None);
        state.changed(Some(account("A")));
        let old = state.input.clone();
        ready(&mut state, "EUW1-1");
        state.changed(None);
        assert!(state.public.stale);
        assert_eq!(state.public.wishes, vec![103001]);
        state.changed(Some(account("B")));
        assert!(state.public.wishes.is_empty());
        assert!(state.public.skins.is_empty());
        state.changed(Some(account("A")));
        assert!(!state.accept(&old, None));
        assert!(state.public.status == Status::Loading);
    }
    #[test]
    fn actualisation_invalide_lecture_en_cours_et_souhaits_obsoletes() {
        let mut state = Runtime::new(None);
        state.changed(Some(account("A")));
        let old = state.input.clone();
        let revision = state.public.revision;
        assert!(state.refresh());
        assert!(!state.accept(&old, None));
        assert!(matches!(state.wish(revision, 103001, true), Err("stale")));
        let revision = state.public.revision;
        assert!(matches!(
            state.wish(revision, 103001, true),
            Err("unavailable")
        ));
    }
    #[test]
    fn echec_de_lecture_et_de_stockage_ne_fabrique_pas_une_collection() {
        let mut state = Runtime::new(None);
        state.changed(Some(account("A")));
        let current = state.input.clone();
        assert!(state.accept(&current, None));
        assert!(state.public.status == Status::Unavailable);
        assert!(state.public.skins.is_empty());
        assert!(state.accept(
            &current,
            Some((
                CollectionSnapshot {
                    account_key: "EUW1-1".into(),
                    skins: vec![]
                },
                Err(())
            ))
        ));
        assert!(state.public.storage_error);
        assert!(state.public.wishes.is_empty());
    }
    #[test]
    fn aucun_souhait_modifiable_hors_connexion() {
        let mut state = Runtime::new(None);
        state.changed(Some(account("A")));
        ready(&mut state, "EUW1-1");
        state.changed(None);
        assert!(matches!(
            state.wish(state.public.revision, 103001, true),
            Err("unavailable")
        ));
        assert!(!state.refresh());
        assert!(state.public.stale);
    }
    #[test]
    fn souhaits_persistes_recharges_et_isoles_par_compte() {
        let directory = std::env::temp_dir().join(format!(
            "olc-collection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let skin = CollectionSkin {
            id: 103001,
            champion_id: 103,
            name: "Ahri".into(),
            ownership: lcu_connector::collection::SkinOwnership::Missing,
            tile_url: None,
            splash_url: None,
            obtainable: None,
            rarity: None,
            series_ids: Vec::new(),
        };
        let mut state = Runtime::new(Some(directory.clone()));
        state.changed(Some(account("A")));
        let input = state.input.clone();
        state.accept(
            &input,
            Some((
                CollectionSnapshot {
                    account_key: "EUW1-1".into(),
                    skins: vec![skin.clone()],
                },
                Ok(BTreeSet::new()),
            )),
        );
        let saved = state.wish(state.public.revision, 103001, true).unwrap();
        assert_eq!(saved.wishes, vec![103001]);
        assert_eq!(
            load_wishes(&directory.join("EUW1-1.json")).unwrap(),
            BTreeSet::from([103001])
        );
        state.changed(Some(account("B")));
        assert!(matches!(
            state.wish(saved.revision, 103001, false),
            Err("stale")
        ));
        let input = state.input.clone();
        state.accept(
            &input,
            Some((
                CollectionSnapshot {
                    account_key: "EUW1-2".into(),
                    skins: vec![skin],
                },
                load_wishes(&directory.join("EUW1-2.json")).map_err(|_| ()),
            )),
        );
        assert!(state.public.wishes.is_empty());
        state.wish(state.public.revision, 103001, true).unwrap();
        state.wish(state.public.revision, 103001, false).unwrap();
        assert!(load_wishes(&directory.join("EUW1-2.json"))
            .unwrap()
            .is_empty());
        assert!(load_wishes(&directory.join("EUW1-1.json"))
            .unwrap()
            .contains(&103001));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
