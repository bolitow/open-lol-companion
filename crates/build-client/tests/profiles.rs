use olc_build_client::{
    profiles::{PlayerError, PlayerMatchesRequest, PlayerRequest},
    BuildClient,
};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
fn request() -> PlayerRequest {
    PlayerRequest {
        platform: "EUW1".into(),
        game_name: "A / B?".into(),
        tag_line: "TEST".into(),
    }
}

#[tokio::test]
async fn conserve_un_solde_de_lp_negatif_accepte_par_api() {
    let mut data = profile();
    data["ranks"] = json!([{"queue_id":420,"status":"ranked","tier":"GOLD","division":"IV","league_points":-5}]);
    let (client, task) = server(200, data).await;
    assert_eq!(
        client.player_profile(request()).await.unwrap().ranks[0].league_points,
        Some(-5)
    );
    task.await.unwrap();
}
/// Contrat partagé avec `@olc/shared` et `olc-api` : le miroir ne doit rien perdre en route.
#[test]
fn le_miroir_du_profil_suit_le_contrat_partage() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../packages/shared/src/contracts/profile.json"
    ))
    .unwrap();
    let profile: olc_build_client::profiles::Profile =
        serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(serde_json::to_value(&profile).unwrap(), golden);
}

#[tokio::test]
async fn lit_victoires_defaites_et_drapeaux_envoyes_par_api() {
    let mut data = profile();
    data["ranks"] = json!([{"queue_id":420,"status":"ranked","tier":"GOLD","division":"IV","league_points":10,
        "wins":3,"losses":2,"hot_streak":true,"veteran":false,"fresh_blood":false,"inactive":false}]);
    let (client, task) = server(200, data).await;
    let rank = client
        .player_profile(request())
        .await
        .unwrap()
        .ranks
        .remove(0);
    assert_eq!((rank.wins, rank.losses), (Some(3), Some(2)));
    assert_eq!(rank.hot_streak, Some(true));
    assert_eq!(rank.veteran, Some(false));
    task.await.unwrap();
}

fn profile() -> Value {
    json!({"platform":"EUW1","game_name":"A / B?","tag_line":"TEST","puuid":"synthetic","profile_icon_id":1,"summoner_level":42,"ranks":[{"queue_id":420,"status":"unranked","tier":null,"division":null,"league_points":null}],"fetched_at":1})
}
fn page() -> Value {
    json!({"platform":"EUW1","game_name":"A / B?","tag_line":"TEST","fetched_at":1,"start":0,"count":10,"next_start":10,"omitted_matches":10,"matches":[]})
}
async fn server(status: u16, body: Value) -> (BuildClient, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        loop {
            let mut buf = [0; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            bytes.extend_from_slice(&buf[..n]);
            if n == 0 || bytes.windows(4).any(|p| p == b"\r\n\r\n") {
                break;
            }
        }
        let first = String::from_utf8_lossy(&bytes)
            .lines()
            .next()
            .unwrap()
            .to_owned();
        let body = body.to_string();
        let wire = format!(
            "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(wire.as_bytes()).await.unwrap();
        first
    });
    (
        BuildClient::new(
            Some(format!("http://{addr}/")),
            Some("synthetic-token".into()),
        )
        .unwrap(),
        task,
    )
}
#[test]
fn refuse_identites_invalides_avant_reseau() {
    for (name, tag) in [("..", "tag"), ("ok", "."), ("a\n", "tag"), ("", "tag")] {
        let mut r = request();
        r.game_name = name.into();
        r.tag_line = tag.into();
        assert_eq!(r.validate(), Err(PlayerError::InvalidRequest));
    }
    let mut r = request();
    r.game_name = "é".repeat(33);
    assert!(r.validate().is_err());
}
#[tokio::test]
async fn encode_chaque_segment_et_lit_le_profil() {
    let (client, task) = server(200, profile()).await;
    assert_eq!(
        client
            .player_profile(request())
            .await
            .unwrap()
            .summoner_level,
        Some(42)
    );
    assert_eq!(
        task.await.unwrap(),
        "GET /v1/profiles/EUW1/A%20%2F%20B%3F/TEST HTTP/1.1"
    );
}
#[tokio::test]
async fn pagination_vide_conserve_le_curseur() {
    let (client, task) = server(200, page()).await;
    let result = client
        .player_matches(PlayerMatchesRequest {
            player: request(),
            start: 0,
            count: 10,
        })
        .await
        .unwrap();
    assert_eq!(result.next_start, Some(10));
    assert!(result.matches.is_empty());
    assert!(task
        .await
        .unwrap()
        .ends_with("/matches?start=0&count=10 HTTP/1.1"));
}
#[tokio::test]
async fn refuse_reponse_autre_compte_et_curseur_incoherent() {
    let mut bad = profile();
    bad["game_name"] = json!("Other");
    let (client, task) = server(200, bad).await;
    assert!(matches!(
        client.player_profile(request()).await,
        Err(PlayerError::InvalidResponse)
    ));
    task.await.unwrap();
    let mut bad = page();
    bad["next_start"] = json!(0);
    let (client, task) = server(200, bad).await;
    assert!(matches!(
        client
            .player_matches(PlayerMatchesRequest {
                player: request(),
                start: 0,
                count: 10
            })
            .await,
        Err(PlayerError::InvalidResponse)
    ));
    task.await.unwrap();
}
#[tokio::test]
async fn erreurs_http_stables_sans_corps_prive() {
    for (status, error) in [
        (400, PlayerError::InvalidRequest),
        (401, PlayerError::Unauthorized),
        (404, PlayerError::NotFound),
        (429, PlayerError::RateLimited),
        (500, PlayerError::Unavailable),
    ] {
        let (client, task) = server(status, json!({"private":"ignored"})).await;
        assert!(matches!(client.player_profile(request()).await,Err(e)if e==error));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn distingue_riot_occupe_des_autres_erreurs_sans_exposer_le_corps() {
    for (status, body, expected) in [
        (503, json!({"error":{"code":"riot_busy"}}), "riot_busy"),
        (503, json!({"error":{"code":"unavailable"}}), "unavailable"),
        (
            503,
            json!({"error":{"code":"riot_busy","private":"ignored"},"padding":"x".repeat(5000)}),
            "unavailable",
        ),
        (500, json!({"error":{"code":"riot_busy"}}), "unavailable"),
    ] {
        let (client, task) = server(status, body).await;
        let error = client.player_profile(request()).await.unwrap_err();
        assert_eq!(serde_json::to_value(error).unwrap(), json!(expected));
        task.await.unwrap();
    }
}
