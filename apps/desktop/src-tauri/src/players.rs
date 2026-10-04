use lcu_connector::{
    players::{
        self, LocalPlayerError, PlayerHistory, PlayerMatch, PlayerProfile, PlayerSource,
        ProfileRank,
    },
    LcuAccount, LcuSession,
};
use olc_build_client::profiles::{PlayerError, PlayerMatchesRequest, PlayerRequest};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Clone, Default)]
struct LocalContext {
    generation: u64,
    connected: bool,
    account: Option<LcuAccount>,
}
impl LocalContext {
    fn update(&mut self, connected: bool, account: Option<LcuAccount>) {
        let account = account.filter(|_| connected);
        if self.connected != connected || self.account != account {
            self.generation = self.generation.saturating_add(1);
            self.connected = connected;
            self.account = account;
        }
    }
    fn local_account(&self, request: &PlayerRequest) -> Result<Option<LcuAccount>, PlayerError> {
        if self.connected && self.account.is_none() {
            return Err(PlayerError::Unavailable);
        }
        Ok(self.account.clone().filter(|a| {
            self.connected
                && a.platform == request.platform
                && a.game_name.to_lowercase() == request.game_name.to_lowercase()
                && a.tag_line.to_lowercase() == request.tag_line.to_lowercase()
        }))
    }
    fn is_current(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.connected == other.connected
            && self.account == other.account
    }
}
#[derive(Default)]
pub struct LocalState(Mutex<LocalContext>);
impl LocalState {
    fn snapshot(&self) -> Result<LocalContext, PlayerError> {
        self.0
            .lock()
            .map(|c| c.clone())
            .map_err(|_| PlayerError::Unavailable)
    }
    fn require_current(&self, context: &LocalContext) -> Result<(), PlayerError> {
        if self.snapshot()?.is_current(context) {
            Ok(())
        } else {
            Err(PlayerError::Unavailable)
        }
    }
}
pub fn lcu_changed(app: &tauri::AppHandle, session: &LcuSession) {
    if let Ok(mut state) = app.state::<LocalState>().0.lock() {
        state.update(session.connected, session.account.clone());
    }
}
fn local_error(error: LocalPlayerError) -> PlayerError {
    match error {
        LocalPlayerError::InvalidRequest => PlayerError::InvalidRequest,
        LocalPlayerError::InvalidResponse => PlayerError::InvalidResponse,
        _ => PlayerError::Unavailable,
    }
}
pub(crate) async fn client() -> Result<lcu_connector::LcuClient, PlayerError> {
    tauri::async_runtime::spawn_blocking(|| {
        let credentials = lcu_connector::discover().map_err(|_| PlayerError::Unavailable)?;
        lcu_connector::LcuClient::new(&credentials).map_err(|_| PlayerError::Unavailable)
    })
    .await
    .map_err(|_| PlayerError::Unavailable)?
}
fn public_profile(profile: olc_build_client::profiles::Profile) -> PlayerProfile {
    // L'identifiant technique reste dans Rust, y compris pour la source publique.
    PlayerProfile {
        platform: profile.platform,
        game_name: profile.game_name,
        tag_line: profile.tag_line,
        profile_icon_id: profile.profile_icon_id,
        summoner_level: profile.summoner_level,
        fetched_at: profile.fetched_at,
        source: PlayerSource::Api,
        ranks: profile
            .ranks
            .into_iter()
            .map(|r| ProfileRank {
                queue_id: r.queue_id,
                status: r.status,
                tier: r.tier,
                division: r.division,
                league_points: r.league_points,
            })
            .collect(),
    }
}
#[tauri::command]
pub async fn player_profile(
    request: PlayerRequest,
    state: tauri::State<'_, crate::api_access::ApiState>,
    local: tauri::State<'_, LocalState>,
) -> Result<PlayerProfile, PlayerError> {
    request.validate()?;
    let context = local.snapshot()?;
    if let Some(account) = context.local_account(&request)? {
        let result = players::read_profile(&client().await?, &account)
            .await
            .map_err(local_error)?;
        local.require_current(&context)?;
        return Ok(result);
    }
    match state.client().await {
        Ok(client) => client.player_profile(request).await.map(public_profile),
        Err(error) => Err(error.into()),
    }
}
/// La pagination conserve sa source : fermer League ne mélange jamais historique local et public.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRequest {
    player: PlayerRequest,
    start: u32,
    count: u32,
    source: String,
}
#[tauri::command]
pub async fn player_matches(
    request: HistoryRequest,
    state: tauri::State<'_, crate::api_access::ApiState>,
    local: tauri::State<'_, LocalState>,
) -> Result<PlayerHistory, PlayerError> {
    request.player.validate()?;
    if request.count == 0 || request.count > 20 || request.start > 10_000 {
        return Err(PlayerError::InvalidRequest);
    }
    if request.source == "lcu" {
        let context = local.snapshot()?;
        let account = context
            .local_account(&request.player)?
            .ok_or(PlayerError::Unavailable)?;
        let result =
            players::read_matches(&client().await?, &account, request.start, request.count)
                .await
                .map_err(local_error)?;
        local.require_current(&context)?;
        return Ok(result);
    }
    if request.source != "api" {
        return Err(PlayerError::InvalidRequest);
    }
    let request = PlayerMatchesRequest {
        player: request.player,
        start: request.start,
        count: request.count,
    };
    let page = match state.client().await {
        Ok(client) => client.player_matches(request).await?,
        Err(error) => return Err(error.into()),
    };
    Ok(PlayerHistory {
        platform: page.platform,
        game_name: page.game_name,
        tag_line: page.tag_line,
        fetched_at: page.fetched_at,
        start: page.start,
        count: page.count,
        next_start: page.next_start,
        omitted_matches: page.omitted_matches,
        source: PlayerSource::Api,
        matches: page
            .matches
            .into_iter()
            .map(|m| PlayerMatch {
                match_id: m.match_id,
                queue_id: m.queue_id,
                patch: Some(m.patch),
                game_start_ms: m.game_start_ms,
                duration_s: m.duration_s,
                champion_id: m.champion_id,
                win: m.win,
                kills: m.kills,
                deaths: m.deaths,
                assists: m.assists,
                items: m.items,
                role: m.role,
            })
            .collect(),
    })
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
    fn request(name: &str) -> PlayerRequest {
        PlayerRequest {
            platform: "EUW1".into(),
            game_name: name.into(),
            tag_line: "TAG".into(),
        }
    }
    #[test]
    fn le_compte_actif_choisit_la_lcu_et_les_autres_profils_le_service_public() {
        let mut context = LocalContext::default();
        context.update(true, Some(account("A")));
        assert!(context.local_account(&request("a")).unwrap().is_some());
        assert!(context.local_account(&request("B")).unwrap().is_none());
        context.update(true, None);
        assert!(matches!(
            context.local_account(&request("A")),
            Err(PlayerError::Unavailable)
        ));
    }
    #[test]
    fn retour_au_meme_compte_ne_valide_pas_une_reponse_d_une_ancienne_connexion() {
        let mut context = LocalContext::default();
        context.update(true, Some(account("A")));
        let old = context.clone();
        context.update(false, None);
        context.update(true, Some(account("A")));
        assert!(!context.is_current(&old));
        let current = context.clone();
        context.update(true, Some(account("A")));
        assert!(context.is_current(&current));
    }
}
