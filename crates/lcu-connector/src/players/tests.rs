use super::*;
use crate::test_support::{mock_client, ExpectedRequest};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const ACCOUNT: &str = "/lol-summoner/v1/current-summoner";
const REGION: &str = "/riotclient/region-locale";
const RANKS: &str = "/lol-ranked/v1/current-ranked-stats";
const HISTORY: &str = "/lol-match-history/v1/products/lol/current-summoner/matches";

fn expected_account() -> LcuAccount {
    LcuAccount {
        platform: "EUW1".into(),
        game_name: "Alpha".into(),
        tag_line: "TEST".into(),
    }
}
fn summoner() -> Value {
    json!({"gameName":"Alpha","tagLine":"TEST","puuid":"private-local-puuid","accountId":12,"summonerId":34,"profileIconId":7,"summonerLevel":123})
}
fn get(path: &str, response: Value) -> ExpectedRequest {
    ExpectedRequest {
        method: "GET",
        path: path.into(),
        body: None,
        status: 200,
        response,
    }
}
fn identity(value: Value, region: &str) -> Vec<ExpectedRequest> {
    vec![get(ACCOUNT, value), get(REGION, json!({"region":region}))]
}
fn around(mut middle: Vec<ExpectedRequest>, after: Value, region: &str) -> Vec<ExpectedRequest> {
    let mut requests = identity(summoner(), "EUW");
    requests.append(&mut middle);
    requests.extend(identity(after, region));
    requests
}
fn ranks() -> Value {
    json!({"queueMap":{"RANKED_SOLO_5x5":{"queueType":"RANKED_SOLO_5x5","tier":"GOLD","division":"II","leaguePoints":-5},"RANKED_FLEX_SR":{"queueType":"RANKED_FLEX_SR","tier":"NONE","division":"NA","leaguePoints":0}}})
}
fn game(id: u64) -> Value {
    json!({"gameId":id,"platformId":"EUW1","gameCreation":1800000000000i64,"gameDuration":1234,"gameMode":"CLASSIC","gameType":"CUSTOM_GAME","gameVersion":"16.19.1.2","queueId":0,
        "participantIdentities":[{"participantId":9,"player":{"puuid":"private-opponent","summonerName":"Do not publish"}},{"participantId":2,"player":{"puuid":"private-local-puuid"}}],
        "participants":[{"participantId":9,"championId":1,"stats":{"win":false}},{"participantId":2,"championId":432,"stats":{"win":true,"kills":2,"deaths":3,"assists":14,"item0":3009,"item1":0,"item2":1001,"item3":0,"item4":0,"item5":0,"item6":3340},"timeline":{"lane":"BOTTOM","role":"DUO_SUPPORT"}}]})
}
fn history(start: u32, games: Vec<Value>) -> Value {
    json!({"platformId":"EUW1","accountId":12,"games":{"gameCount":games.len(),"gameIndexBegin":start,"gameIndexEnd":start.saturating_add(games.len().saturating_sub(1) as u32),"games":games}})
}
async fn read_page(
    value: Value,
    start: u32,
    count: u32,
) -> Result<PlayerHistory, LocalPlayerError> {
    let path = format!("{HISTORY}?begIndex={start}&endIndex={}", start + count - 1);
    let (client, server) = mock_client(around(vec![get(&path, value)], summoner(), "EUW")).await;
    let result = read_matches(&client, &expected_account(), start, count).await;
    // La validation du compte est effectuée même si la projection est rejetée.
    server.await.unwrap();
    result
}

#[tokio::test]
async fn projette_le_profil_local_et_des_rangs_explicites_sans_identifiant_technique() {
    let (client, server) = mock_client(around(vec![get(RANKS, ranks())], summoner(), "EUW")).await;
    let profile = read_profile(&client, &expected_account()).await.unwrap();
    assert_eq!(profile.source, PlayerSource::Lcu);
    assert_eq!(profile.summoner_level, Some(123));
    assert_eq!(profile.profile_icon_id, Some(7));
    assert_eq!(profile.ranks.len(), 2);
    assert_eq!(profile.ranks[0].queue_id, 420);
    assert_eq!(profile.ranks[0].league_points, Some(-5));
    assert_eq!(profile.ranks[1].status, "unranked");
    assert!(profile.ranks[1].tier.is_none());
    let text = serde_json::to_string(&profile).unwrap();
    for forbidden in ["puuid", "accountId", "summonerId", "private-local"] {
        assert!(!text.contains(forbidden));
    }
    server.await.unwrap();
}

#[tokio::test]
async fn conserve_le_profil_si_les_rangs_sont_indisponibles_sans_inventer_non_classe() {
    for status in [200, 503] {
        let mut rank = get(RANKS, json!({"queueMap":{}}));
        rank.status = status;
        let (client, server) = mock_client(around(vec![rank], summoner(), "EUW")).await;
        let profile = read_profile(&client, &expected_account()).await.unwrap();
        assert!(profile.ranks.is_empty());
        assert_eq!(profile.summoner_level, Some(123));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn joint_le_participant_local_par_identite_et_conserve_la_personnalisee() {
    let result = read_page(history(0, vec![game(51)]), 0, 10).await.unwrap();
    assert_eq!(result.source, PlayerSource::Lcu);
    assert_eq!(result.matches.len(), 1);
    let local = &result.matches[0];
    assert_eq!(local.match_id, "EUW1_51");
    assert_eq!(local.queue_id, 0);
    assert_eq!(local.champion_id, 432);
    assert!(local.win);
    assert_eq!(
        (local.kills, local.deaths, local.assists),
        (Some(2), Some(3), Some(14))
    );
    assert_eq!(local.items, vec![3009, 1001, 3340]);
    assert_eq!(local.patch.as_deref(), Some("16.19"));
    assert_eq!(local.game_start_ms, 1800000000000);
    assert_eq!(local.duration_s, 1234);
    assert_eq!(local.role.as_deref(), Some("UTILITY"));
    assert_eq!(result.next_start, None);
    let text = serde_json::to_string(&result).unwrap();
    for forbidden in [
        "puuid",
        "participantId",
        "accountId",
        "private-",
        "Do not publish",
    ] {
        assert!(!text.contains(forbidden));
    }
}

#[tokio::test]
async fn accepte_un_patch_absent_sans_l_inventer() {
    for patch in [Value::Null, json!(""), json!("unknown")] {
        let mut row = game(51);
        row["gameVersion"] = patch;
        let result = read_page(history(0, vec![row]), 0, 10).await.unwrap();
        assert_eq!(result.matches[0].patch, None);
    }
}

#[tokio::test]
async fn refuse_identites_ambigues_statistiques_malformees_et_doublons() {
    for fault in 0..5 {
        let mut row = game(51);
        match fault {
            0 => {
                let same = row["participantIdentities"][1].clone();
                row["participantIdentities"]
                    .as_array_mut()
                    .unwrap()
                    .push(same);
            }
            1 => {
                let same = row["participants"][1].clone();
                row["participants"].as_array_mut().unwrap().push(same);
            }
            2 => row["participants"][1]["stats"]["win"] = Value::Null,
            3 => row["gameCreation"] = json!(-1),
            _ => row["platformId"] = json!("NA1"),
        }
        assert!(matches!(
            read_page(history(0, vec![row]), 0, 10).await,
            Err(LocalPlayerError::InvalidResponse)
        ));
    }
    assert!(matches!(
        read_page(history(0, vec![game(51), game(51)]), 0, 10).await,
        Err(LocalPlayerError::InvalidResponse)
    ));
}

#[tokio::test]
async fn avance_uniquement_une_page_pleine_coherente_et_refuse_un_cache_ancien() {
    let result = read_page(history(10, vec![game(51), game(52)]), 10, 2)
        .await
        .unwrap();
    assert_eq!(result.next_start, Some(12));
    assert_eq!((result.start, result.count), (10, 2));
    assert!(matches!(
        read_page(history(0, vec![game(51)]), 10, 2).await,
        Err(LocalPlayerError::InvalidResponse)
    ));
    let mut malformed = history(10, vec![game(51), game(52)]);
    malformed["games"]["gameIndexEnd"] = json!(12);
    assert!(matches!(
        read_page(malformed, 10, 2).await,
        Err(LocalPlayerError::InvalidResponse)
    ));
    let empty = read_page(history(0, vec![]), 0, 10).await.unwrap();
    assert!(empty.matches.is_empty());
    assert_eq!(empty.next_start, None);
}

#[tokio::test]
async fn refuse_un_compte_ou_une_region_modifies_pendant_les_lectures() {
    for change in ["puuid", "gameName", "region"] {
        let mut after = summoner();
        if change != "region" {
            after[change] = json!("Other");
        }
        let region = if change == "region" { "NA" } else { "EUW" };
        let (client, server) = mock_client(around(vec![get(RANKS, ranks())], after, region)).await;
        assert!(matches!(
            read_profile(&client, &expected_account()).await,
            Err(LocalPlayerError::AccountChanged)
        ));
        server.await.unwrap();
    }
    let mut after = summoner();
    after["puuid"] = json!("Other");
    let (client, server) = mock_client(around(
        vec![get(
            &format!("{HISTORY}?begIndex=0&endIndex=9"),
            history(0, vec![game(51)]),
        )],
        after,
        "EUW",
    ))
    .await;
    assert!(matches!(
        read_matches(&client, &expected_account(), 0, 10).await,
        Err(LocalPlayerError::AccountChanged)
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn ne_confond_pas_le_compteur_client_et_la_longueur_de_page() {
    let mut page = history(10, vec![game(51), game(52)]);
    page["games"]["gameCount"] = json!(99);
    let result = read_page(page, 10, 2).await.unwrap();
    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.next_start, Some(12));
    let mut short = history(10, vec![game(51)]);
    short["games"]["gameCount"] = json!(99);
    assert_eq!(read_page(short, 10, 2).await.unwrap().next_start, None);
}

#[tokio::test]
async fn rejette_les_bornes_et_un_autre_compte_avant_lecture_de_l_historique() {
    let (client, server) = mock_client(vec![]).await;
    for (start, count) in [(10001, 10), (0, 0), (0, 21)] {
        assert!(matches!(
            read_matches(&client, &expected_account(), start, count).await,
            Err(LocalPlayerError::InvalidRequest)
        ));
    }
    server.await.unwrap();
    let mut wrong = expected_account();
    wrong.game_name = "Other".into();
    let (client, server) = mock_client(identity(summoner(), "EUW")).await;
    assert!(matches!(
        read_matches(&client, &wrong, 0, 10).await,
        Err(LocalPlayerError::AccountChanged)
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn borne_a_dix_secondes_la_somme_des_requetes_locales() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let credentials =
        crate::Credentials::from_lockfile(&format!("LeagueClient:1:{port}:test:http")).unwrap();
    let client = LcuClient::new(&credentials).unwrap();
    let server = tokio::spawn(async move {
        for value in [summoner(), json!({"region":"EUW"}), ranks()] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let mut chunk = [0; 1024];
                let read = stream.read(&mut chunk).await.unwrap();
                if read == 0 {
                    return;
                }
                request.extend_from_slice(&chunk[..read]);
                assert!(request.len() <= 8192);
            }
            tokio::time::sleep(Duration::from_secs(4)).await;
            let body = value.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            if stream.write_all(response.as_bytes()).await.is_err() {
                return;
            }
        }
    });
    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(12),
        read_profile(&client, &expected_account()),
    )
    .await
    .unwrap();
    server.abort();
    assert!(matches!(result, Err(LocalPlayerError::Unavailable)));
    assert!(started.elapsed() >= Duration::from_secs(9));
    assert!(started.elapsed() < Duration::from_secs(12));
}
