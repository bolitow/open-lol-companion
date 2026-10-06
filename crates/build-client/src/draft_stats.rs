//! Projection de la tierlist, complète et cohérente entre les six rôles publics.
use crate::{BuildClient, BuildError, BuildMeta, BuildRequest};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftStatsRequest {
    pub patch: String,
    pub platform: String,
    pub queue: u32,
    pub rank: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftChampionStats {
    pub patch: String,
    pub platform_id: String,
    pub queue_id: u32,
    pub rank: String,
    pub role: String,
    pub champion_id: u32,
    pub games: u64,
    pub wins: u64,
    pub win_rate: Option<f64>,
    pub pick_rate: Option<f64>,
    pub win_rate_lower_bound: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct DraftStatsReport {
    pub meta: BuildMeta,
    pub entries: Vec<DraftChampionStats>,
}
const ROLES: [&str; 6] = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY", "UNKNOWN"];
const MAX_PER_ROLE: usize = 2000;
const MAX_TOTAL: usize = 12000;
impl DraftStatsRequest {
    fn validation_request(&self) -> BuildRequest {
        BuildRequest {
            champion_id: 1,
            patch: self.patch.clone(),
            platform: self.platform.clone(),
            queue: self.queue,
            role: "UNKNOWN".into(),
            rank: self.rank.clone(),
        }
    }
    pub fn validate(&self) -> Result<(), BuildError> {
        self.validation_request().validate()
    }
}
#[derive(Deserialize)]
struct Page {
    meta: serde_json::Value,
    query: serde_json::Value,
    total: usize,
    entries: Vec<DraftChampionStats>,
}
impl DraftChampionStats {
    fn check(
        &self,
        request: &DraftStatsRequest,
        role: &str,
        min_games: u32,
    ) -> Result<(), BuildError> {
        let rate = |value: Option<f64>| {
            value.map_or(true, |v| v.is_finite() && (0.0..=100.0).contains(&v))
        };
        if self.patch != request.patch
            || self.platform_id != request.platform
            || self.queue_id != request.queue
            || self.rank != request.rank
            || self.role != role
            || self.champion_id == 0
            || self.games > 9_007_199_254_740_991
            || self.wins > self.games
            || (self.games < u64::from(min_games) && self.win_rate.is_some())
            || !rate(self.win_rate)
            || !rate(self.pick_rate)
            || !rate(self.win_rate_lower_bound)
            || self
                .win_rate_lower_bound
                .is_some_and(|low| self.win_rate.map_or(true, |win| low > win))
        {
            return Err(BuildError::InvalidResponse);
        }
        Ok(())
    }
}
impl BuildClient {
    /// Charge les six rôles d’une même publication ; aucune sélection de champion ou de file implicite.
    pub async fn draft_stats(
        &self,
        request: DraftStatsRequest,
    ) -> Result<DraftStatsReport, BuildError> {
        request.validate()?;
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            let _permit = self
                .slots
                .acquire()
                .await
                .map_err(|_| BuildError::Unavailable)?;
            let mut entries = Vec::new();
            let mut publication = None;
            let mut result_meta = None;
            let mut seen = std::collections::HashSet::new();

            for role in ROLES {
                let mut offset = 0;
                let mut expected_total = None;
                loop {
                    let mut url = self
                        .base
                        .join("v1/tierlist")
                        .map_err(|_| BuildError::InvalidConfiguration)?;
                    url.query_pairs_mut().extend_pairs([
                        ("patch", request.patch.clone()),
                        ("platform", request.platform.clone()),
                        ("queue", request.queue.to_string()),
                        ("rank", request.rank.clone()),
                        ("role", role.to_string()),
                        ("offset", offset.to_string()),
                        ("limit", crate::PAGE_SIZE.to_string()),
                    ]);
                    let expected = serde_json::json!({
                        "patch": request.patch, "platform": request.platform,
                        "queue": request.queue, "rank": request.rank,
                        "role": role, "offset": offset, "limit": crate::PAGE_SIZE,
                    });
                    let mut response = self
                        .http
                        .get(url)
                        .send()
                        .await
                        .map_err(|_| BuildError::Unavailable)?;
                    match response.status().as_u16() {
                        200 => {}
                        400 => return Err(BuildError::InvalidRequest),
                        401 | 403 => return Err(BuildError::Unauthorized),
                        429 => return Err(BuildError::RateLimited),
                        _ => return Err(BuildError::Unavailable),
                    }
                    if response
                        .content_length()
                        .is_some_and(|n| n > crate::MAX_BYTES as u64)
                    {
                        return Err(BuildError::InvalidResponse);
                    }
                    let mut bytes = Vec::new();
                    while let Some(chunk) = response
                        .chunk()
                        .await
                        .map_err(|_| BuildError::Unavailable)?
                    {
                        if chunk.len() > crate::MAX_BYTES.saturating_sub(bytes.len()) {
                            return Err(BuildError::InvalidResponse);
                        }
                        bytes.extend_from_slice(&chunk);
                    }
                    let page: Page =
                        serde_json::from_slice(&bytes).map_err(|_| BuildError::InvalidResponse)?;
                    if page.query != expected
                        || page.total > MAX_PER_ROLE
                        || offset > page.total
                        || page.entries.len() != crate::PAGE_SIZE.min(page.total - offset)
                        || entries.len() + page.entries.len() > MAX_TOTAL
                    {
                        return Err(BuildError::InvalidResponse);
                    }
                    if publication.as_ref().is_some_and(|meta| meta != &page.meta)
                        || expected_total.is_some_and(|n| n != page.total)
                    {
                        return Err(BuildError::ChangedSnapshot);
                    }
                    let meta: BuildMeta = serde_json::from_value(page.meta.clone())
                        .map_err(|_| BuildError::InvalidResponse)?;
                    if meta.min_games == 0
                        || meta.source_snapshot_at.is_empty()
                        || meta.source_snapshot_at.len() > 64
                        || meta.published_at.is_empty()
                        || meta.published_at.len() > 64
                    {
                        return Err(BuildError::InvalidResponse);
                    }
                    meta.check_population(&request.validation_request())?;
                    for row in &page.entries {
                        row.check(&request, role, meta.min_games)?;
                        if !seen.insert((row.champion_id, row.role.clone())) {
                            return Err(BuildError::InvalidResponse);
                        }
                    }
                    offset += page.entries.len();
                    entries.extend(page.entries);
                    publication = Some(page.meta);
                    result_meta = Some(meta);
                    expected_total = Some(page.total);
                    if offset == page.total {
                        break;
                    }
                }
            }
            Ok(DraftStatsReport {
                meta: result_meta.ok_or(BuildError::InvalidResponse)?,
                entries,
            })
        })
        .await
        .map_err(|_| BuildError::Unavailable)?
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn request() -> DraftStatsRequest {
        DraftStatsRequest {
            patch: "16.19".into(),
            platform: "EUW1".into(),
            queue: 420,
            rank: "ALL".into(),
        }
    }
    fn row(role: &str, id: u32) -> Value {
        json!({"patch":"16.19","platform_id":"EUW1","queue_id":420,"rank":"ALL","role":role,"champion_id":id,"games":100,"wins":50,"win_rate":50.0,"pick_rate":10.0,"win_rate_lower_bound":40.0})
    }
    fn page(role: &str, offset: usize, total: usize, entries: Vec<Value>) -> Value {
        json!({"meta":{"source_snapshot_at":"2026-10-01T12:00:00Z","published_at":"2026-10-01T12:05:00Z","min_games":50},"query":{"patch":"16.19","platform":"EUW1","queue":420,"rank":"ALL","role":role,"offset":offset,"limit":200},"total":total,"entries":entries,"bans":[]})
    }
    async fn server(pages: Vec<Value>) -> (BuildClient, tokio::task::JoinHandle<Vec<String>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for page in pages {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0; 8192];
                let n = socket.read(&mut bytes).await.unwrap();
                let text = String::from_utf8_lossy(&bytes[..n]);
                assert!(text
                    .to_ascii_lowercase()
                    .contains("authorization: bearer test-token"));
                requests.push(text.lines().next().unwrap().to_string());
                let body = page.to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
            requests
        });
        (
            BuildClient::new(Some(url), Some("test-token".into())).unwrap(),
            task,
        )
    }
    #[tokio::test]
    async fn lit_six_roles_et_toutes_les_pages_sans_melanger() {
        let mut pages = vec![
            page("TOP", 0, 201, (1..=200).map(|id| row("TOP", id)).collect()),
            page("TOP", 200, 201, vec![row("TOP", 201)]),
        ];
        pages.extend(
            ROLES[1..]
                .iter()
                .map(|role| page(role, 0, 1, vec![row(role, 1)])),
        );
        let (client, server) = server(pages).await;
        let report = client.draft_stats(request()).await.unwrap();
        assert_eq!(report.entries.len(), 206);
        assert_eq!(report.meta.min_games, 50);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 7);
        assert!(requests[1].contains("offset=200"));
        assert!(requests.iter().all(|r| r.starts_with("GET /v1/tierlist?")));
    }
    #[tokio::test]
    async fn transmet_la_definition_du_pickrate_de_la_tierlist() {
        let definition = "champion_matches / bucket_matches * 100";
        let pages = ROLES
            .iter()
            .map(|role| {
                let mut value = page(role, 0, 0, vec![]);
                value["meta"]["pick_rate_definition"] = json!(definition);
                value
            })
            .collect();
        let (client, server) = server(pages).await;
        let report = serde_json::to_value(client.draft_stats(request()).await.unwrap()).unwrap();
        assert_eq!(report["meta"]["pick_rate_definition"], definition);
        server.await.unwrap();
    }
    #[tokio::test]
    async fn refuse_doublons_snapshot_et_scope_incoherents() {
        let duplicate = page("TOP", 0, 2, vec![row("TOP", 1), row("TOP", 1)]);
        let mut scope = page("TOP", 0, 1, vec![row("TOP", 1)]);
        scope["entries"][0]["platform_id"] = json!("NA1");
        let first = page("TOP", 0, 0, vec![]);
        let mut changed = page("JUNGLE", 0, 0, vec![]);
        changed["meta"]["published_at"] = json!("changed");
        for (pages, error) in [
            (vec![duplicate], BuildError::InvalidResponse),
            (vec![scope], BuildError::InvalidResponse),
            (vec![first, changed], BuildError::ChangedSnapshot),
        ] {
            let (client, server) = server(pages).await;
            assert_eq!(client.draft_stats(request()).await.unwrap_err(), error);
            server.abort();
        }
    }
    #[tokio::test]
    async fn refuse_query_total_et_taux_invalides() {
        let valid = page("TOP", 0, 1, vec![row("TOP", 1)]);
        for (pointer, value) in [
            ("/query/offset", json!(1)),
            ("/total", json!(2001)),
            ("/entries/0/wins", json!(101)),
            ("/entries/0/games", json!(9007199254740992_u64)),
            ("/entries/0/win_rate", json!(101)),
            ("/entries/0/win_rate_lower_bound", json!(51)),
            ("/entries/0/role", json!("JUNGLE")),
            (
                "/meta/coverage",
                json!([{"patch":"16.20","platform_id":"EUW1","queue_id":420,"ranked_participations":0}]),
            ),
        ] {
            let mut bad = valid.clone();
            bad.pointer_mut(pointer)
                .map(|p| *p = value.clone())
                .unwrap_or_else(|| {
                    bad["meta"]["coverage"] = value;
                });
            let (client, server) = server(vec![bad]).await;
            assert_eq!(
                client.draft_stats(request()).await.unwrap_err(),
                BuildError::InvalidResponse,
                "{pointer}"
            );
            server.abort();
        }
    }
    #[tokio::test]
    async fn refuse_doublon_entre_pages_et_changement_meta_non_projete() {
        let first = page("TOP", 0, 201, (1..=200).map(|id| row("TOP", id)).collect());
        let mut changed = page("JUNGLE", 0, 0, vec![]);
        changed["meta"]["tier_method"] = json!("changed");
        for (pages, error) in [
            (
                vec![first, page("TOP", 200, 201, vec![row("TOP", 1)])],
                BuildError::InvalidResponse,
            ),
            (
                vec![page("TOP", 0, 0, vec![]), changed],
                BuildError::ChangedSnapshot,
            ),
        ] {
            let (client, server) = server(pages).await;
            assert_eq!(client.draft_stats(request()).await.unwrap_err(), error);
            server.abort();
        }
    }
    #[test]
    fn dimensions_validees_sans_restriction_solo_flex() {
        let mut request = request();
        request.queue = 1700;
        assert!(request.validate().is_ok());
        request.patch = "16.19.1".into();
        assert_eq!(request.validate(), Err(BuildError::InvalidRequest));
    }
    #[tokio::test]
    async fn refuse_taux_publie_sous_le_seuil_dechantillon() {
        let mut insufficient = page("TOP", 0, 1, vec![row("TOP", 1)]);
        insufficient["meta"]["min_games"] = json!(101);
        let (client, server) = server(vec![insufficient]).await;
        assert_eq!(
            client.draft_stats(request()).await.unwrap_err(),
            BuildError::InvalidResponse
        );
        server.abort();
    }
}
