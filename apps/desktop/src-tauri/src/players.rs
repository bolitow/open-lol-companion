use olc_build_client::profiles::{
    PlayerError, PlayerMatchesRequest, PlayerRequest, Profile, ProfileMatches,
};

#[tauri::command]
pub async fn player_profile(
    request: PlayerRequest,
    state: tauri::State<'_, super::BuildState>,
) -> Result<Profile, PlayerError> {
    match state.inner() {
        Ok(client) => client.player_profile(request).await,
        Err(error) => Err((*error).into()),
    }
}
#[tauri::command]
pub async fn player_matches(
    request: PlayerMatchesRequest,
    state: tauri::State<'_, super::BuildState>,
) -> Result<ProfileMatches, PlayerError> {
    match state.inner() {
        Ok(client) => client.player_matches(request).await,
        Err(error) => Err((*error).into()),
    }
}
