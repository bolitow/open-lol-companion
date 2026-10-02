//! Liste sociale du client actif, indépendante des lectures Live Client et de l'API publique.
use lcu_connector::{
    friends::{read_friends, Friend, FriendsSnapshot, FriendsStatus},
    LcuAccount, LcuSession,
};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Emitter, Manager};
use tokio::sync::watch;

#[derive(Clone, Serialize)]
pub struct FriendsState {
    revision: u64,
    status: FriendsStatus,
    items: Vec<Friend>,
}
#[derive(Clone, Default, PartialEq, Eq)]
struct Input {
    generation: u64,
    account: Option<LcuAccount>,
}
pub(super) struct Runtime {
    input: Input,
    public: FriendsState,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            input: Input::default(),
            public: FriendsState {
                revision: 0,
                status: FriendsStatus::Disconnected,
                items: vec![],
            },
        }
    }
}
impl Runtime {
    fn account_changed(&mut self, account: Option<LcuAccount>) -> bool {
        if self.input.account == account {
            return false;
        }
        self.input.generation = self.input.generation.saturating_add(1);
        self.input.account = account;
        self.public.revision = self.public.revision.saturating_add(1);
        self.public.items.clear();
        self.public.status = if self.input.account.is_some() {
            FriendsStatus::Loading
        } else {
            FriendsStatus::Disconnected
        };
        true
    }
    fn accept(&mut self, input: &Input, snapshot: FriendsSnapshot) -> bool {
        if &self.input != input {
            return false;
        }
        self.public.revision = self.public.revision.saturating_add(1);
        self.public.status = snapshot.status;
        self.public.items = snapshot.items;
        true
    }
}
type Shared = Arc<Mutex<Runtime>>;
struct Control(watch::Sender<Input>);

#[tauri::command]
pub fn friends_state(state: tauri::State<'_, Shared>) -> Result<FriendsState, &'static str> {
    state
        .lock()
        .map(|s| s.public.clone())
        .map_err(|_| "unavailable")
}

pub fn lcu_changed(app: &tauri::AppHandle, session: &LcuSession) {
    let account = session.account.clone().filter(|_| session.connected);
    let shared = app.state::<Shared>();
    if let Ok(mut state) = shared.lock() {
        if state.account_changed(account) {
            app.state::<Control>().0.send_replace(state.input.clone());
            let _ = app.emit_to("main", "friends-state", &state.public);
        }
    };
}

async fn read(account: &LcuAccount) -> FriendsSnapshot {
    let client = tauri::async_runtime::spawn_blocking(|| {
        let credentials = lcu_connector::discover().ok()?;
        lcu_connector::LcuClient::new(&credentials).ok()
    })
    .await
    .ok()
    .flatten();
    match client {
        Some(client) => read_friends(&client, account).await,
        None => FriendsSnapshot {
            status: FriendsStatus::Unavailable,
            items: vec![],
        },
    }
}
pub fn setup(app: &tauri::AppHandle) {
    let state = Arc::new(Mutex::new(Runtime::default()));
    let (sender, mut input) = watch::channel(Input::default());
    app.manage(state.clone());
    app.manage(Control(sender));
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let current = input.borrow_and_update().clone();
            if let Some(account) = &current.account {
                let result = tokio::select! { biased;
                    changed=input.changed()=>{ if changed.is_err(){break;} continue; },
                    result=read(account)=>result,
                };
                if let Ok(mut state) = state.lock() {
                    if state.accept(&current, result) {
                        let _ = handle.emit_to("main", "friends-state", &state.public);
                    }
                }
                tokio::select! { biased;
                    changed=input.changed()=>{if changed.is_err(){break;}},
                    _=tokio::time::sleep(Duration::from_secs(30))=>{},
                }
            } else if input.changed().await.is_err() {
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
            tag_line: "TAG".into(),
        }
    }
    #[test]
    fn changement_de_compte_efface_et_refuse_une_ancienne_reponse_meme_apres_retour() {
        let mut state = Runtime::default();
        assert!(state.account_changed(Some(account("A"))));
        let old = state.input.clone();
        assert!(!state.account_changed(Some(account("A"))));
        assert!(state.account_changed(None));
        assert!(state.account_changed(Some(account("A"))));
        assert!(!state.accept(
            &old,
            FriendsSnapshot {
                status: FriendsStatus::Ready,
                items: vec![]
            }
        ));
        let current = state.input.clone();
        assert!(state.accept(
            &current,
            FriendsSnapshot {
                status: FriendsStatus::Ready,
                items: vec![]
            }
        ));
        assert!(matches!(state.public.status, FriendsStatus::Ready));
    }
}
