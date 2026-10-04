use super::*;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn attend_un_creneau_pour_le_dernier_choix_apres_des_changements_rapides() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for _ in 0..5 {
            let (mut socket, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut buf = [0; 8192];
                let count = socket.read(&mut buf).await.unwrap();
                assert!(count > 0);
                tokio::time::sleep(Duration::from_millis(70)).await;
                let body = page(0, 0, vec![]).to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            });
        }
    });
    let client = Arc::new(BuildClient::new(Some(url), Some("test-token".into())).unwrap());
    let mut jobs = tokio::task::JoinSet::new();
    for _ in 0..5 {
        let client = client.clone();
        jobs.spawn(async move { client.builds(request()).await });
    }
    let mut succeeded = 0;
    while let Some(result) = jobs.join_next().await {
        if result.unwrap().is_ok() {
            succeeded += 1;
        }
    }
    server.abort();
    assert_eq!(succeeded, 5);
}

fn request() -> BuildRequest {
    serde_json::from_value(json!({"champion_id":103,"patch":"16.19","platform":"EUW1","queue":420,"role":"MIDDLE","rank":"ALL"})).unwrap()
}
fn variant(category: &str, selection: Vec<u32>) -> Value {
    json!({"champion_id":103,"patch":"16.19","platform_id":"EUW1","queue_id":420,"role":"MIDDLE","rank":"ALL","category":category,"selection":selection,"games":120,"wins":60,"performance_available":true,"population":200,"pick_rate":60.0,"win_rate":50.0})
}
fn page(offset: usize, total: usize, rows: Vec<Value>) -> Value {
    json!({"champion_id":103,"meta":{"source_snapshot_at":"2026-10-01T12:00:00Z","published_at":"2026-10-01T12:05:00Z","min_games":100},"query":{"patch":"16.19","platform":"EUW1","queue":420,"role":"MIDDLE","rank":"ALL","offset":offset,"limit":200},"total":total,"builds":rows})
}
async fn server(responses: Vec<(u16, Value)>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let job = tokio::spawn(async move {
        let mut requests = vec![];
        for (status, value) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![0; 16384];
            let count = socket.read(&mut bytes).await.unwrap();
            let text = String::from_utf8_lossy(&bytes[..count]);
            requests.push(text.lines().next().unwrap().to_owned());
            assert!(text
                .to_lowercase()
                .contains("authorization: bearer test-token"));
            let body = value.to_string();
            socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
        requests
    });
    (address, job)
}

#[test]
fn refuse_les_origines_non_sures_sans_exposer_la_configuration() {
    assert!(matches!(
        BuildClient::new(None, None),
        Err(BuildError::NotConfigured)
    ));
    for url in [
        "http://example.com",
        "https://user:pass@example.com",
        "https://example.com/?token=a",
        "file:///tmp/test",
        "https://example.com/path",
        "https://example.com/#fragment",
    ] {
        assert!(matches!(
            BuildClient::new(Some(url.into()), Some("test-token".into())),
            Err(BuildError::InvalidConfiguration)
        ));
    }
    assert!(BuildClient::new(
        Some("https://example.com".into()),
        Some("test-token".into())
    )
    .is_ok());
    assert!(BuildClient::new(
        Some("http://127.0.0.1:3030".into()),
        Some("test-token".into())
    )
    .is_ok());
}

#[test]
fn refuse_un_perimetre_invalide() {
    let req = request();
    assert!(req.validate().is_ok());
    for (key, value) in [
        ("champion_id", json!(0)),
        ("patch", json!("16.19.1")),
        ("platform", json!("EU")),
        ("role", json!("MID")),
        ("rank", json!("FAKE")),
        ("queue", json!(0)),
    ] {
        let mut value_req = serde_json::to_value(&req).unwrap();
        value_req[key] = value;
        let req: BuildRequest = serde_json::from_value(value_req).unwrap();
        assert_eq!(req.validate(), Err(BuildError::InvalidRequest));
    }
}

#[tokio::test]
async fn charge_toutes_les_categories_sans_perdre_les_fragments_repetes() {
    let first = (0..200).map(|id| variant("item", vec![id + 1])).collect();
    let last = variant(
        "runes",
        vec![
            8000, 8005, 9111, 9104, 8014, 8200, 8224, 8234, 5008, 5008, 5011,
        ],
    );
    let (url, job) = server(vec![
        (200, page(0, 201, first)),
        (200, page(200, 201, vec![last])),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    assert_eq!(report.builds.len(), 201);
    assert_eq!(report.builds[200].selection[8..10], [5008, 5008]);
    let requests = job.await.unwrap();
    assert!(requests[0].contains("limit=200"));
    assert!(requests[1].contains("offset=200"));
}

#[tokio::test]
async fn refuse_un_changement_de_snapshot_entre_deux_pages() {
    let mut second = page(200, 201, vec![variant("runes", vec![1])]);
    second["meta"]["published_at"] = json!("2026-10-01T13:00:00Z");
    let (url, job) = server(vec![
        (
            200,
            page(
                0,
                201,
                (0..200).map(|id| variant("item", vec![id + 1])).collect(),
            ),
        ),
        (200, second),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    assert_eq!(
        client.builds(request()).await.unwrap_err(),
        BuildError::ChangedSnapshot
    );
    job.await.unwrap();
}

#[tokio::test]
async fn refuse_les_reponses_d_un_autre_champion_ou_perimetre() {
    let mut bad = variant("item", vec![1055]);
    bad["champion_id"] = json!(222);
    let (url, job) = server(vec![(200, page(0, 1, vec![bad]))]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    assert_eq!(
        client.builds(request()).await.unwrap_err(),
        BuildError::InvalidResponse
    );
    job.await.unwrap();
}

#[tokio::test]
async fn ne_transmet_ni_corps_d_erreur_ni_statistiques_sous_seuil() {
    let mut low = variant("item", vec![1055]);
    low["games"] = json!(2);
    low["wins"] = json!(1);
    let (url, job) = server(vec![
        (401, json!({"private":"contenu-a-ne-pas-transmettre"})),
        (200, page(0, 1, vec![low])),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    assert_eq!(
        serde_json::to_string(&client.builds(request()).await.unwrap_err()).unwrap(),
        "\"unauthorized\""
    );
    let report = client.builds(request()).await.unwrap();
    assert!(report.builds[0].win_rate.is_none());
    assert!(report.builds[0].pick_rate.is_none());
    job.await.unwrap();
}

#[tokio::test]
async fn transmet_la_borne_wilson_des_etapes_et_la_masque_comme_le_winrate() {
    let mut core = variant("core", vec![6672, 3031, 3089]);
    core["win_rate_lower_bound"] = json!(41.2);
    let mut low = variant("starter", vec![1055, 2003]);
    low["games"] = json!(2);
    low["wins"] = json!(1);
    low["win_rate_lower_bound"] = json!(10.0);
    let mut arena = variant("boots", vec![3006]);
    arena["performance_available"] = json!(false);
    arena["win_rate_lower_bound"] = json!(30.0);
    let legacy = variant("item", vec![1055]);
    let mut invalid = variant("core", vec![1, 2, 3]);
    invalid["win_rate_lower_bound"] = json!(120.0);
    let (url, job) = server(vec![
        (200, page(0, 4, vec![core, low, arena, legacy])),
        (200, page(0, 1, vec![invalid])),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    let bounds: Vec<_> = report
        .builds
        .iter()
        .map(|b| b.win_rate_lower_bound)
        .collect();
    assert_eq!(bounds, vec![Some(41.2), None, None, None]);
    // Le core garde son ordre d'achat, sans tri.
    assert_eq!(report.builds[0].selection, vec![6672, 3031, 3089]);
    assert_eq!(
        client.builds(request()).await.unwrap_err(),
        BuildError::InvalidResponse
    );
    job.await.unwrap();
}

#[tokio::test]
async fn accepte_une_variante_sans_victoire_dont_la_borne_wilson_est_nulle() {
    // Contrat avec le collecteur : 0 victoire sur 118 parties publie une borne de 0,
    // jamais une valeur flottante infime négative qui ferait rejeter toute la page.
    let mut afk = variant("starter", vec![]);
    afk["games"] = json!(118);
    afk["wins"] = json!(0);
    afk["win_rate"] = json!(0.0);
    afk["win_rate_lower_bound"] = json!(0.0);
    let core = variant("core", vec![6672, 3031, 3089]);
    let (url, job) = server(vec![(200, page(0, 2, vec![afk, core]))]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    assert_eq!(report.builds.len(), 2);
    assert_eq!(report.builds[0].wins, Some(0));
    assert_eq!(report.builds[0].win_rate_lower_bound, Some(0.0));
    job.await.unwrap();
}

#[tokio::test]
async fn transmet_l_intervalle_et_l_ecart_au_champion_avec_les_memes_masques() {
    let mut sure = variant("core", vec![6672, 3031, 3089]);
    sure["win_rate_lower_bound"] = json!(41.2);
    sure["win_rate_upper_bound"] = json!(58.9);
    sure["win_rate_delta"] = json!(-2.5);
    let mut rare = variant("starter", vec![1055]);
    rare["games"] = json!(2);
    rare["wins"] = json!(1);
    rare["win_rate_upper_bound"] = json!(90.0);
    rare["win_rate_delta"] = json!(4.0);
    let mut arena = variant("boots", vec![3006]);
    arena["performance_available"] = json!(false);
    arena["win_rate_upper_bound"] = json!(70.0);
    arena["win_rate_delta"] = json!(1.0);
    let legacy = variant("item", vec![1055]);
    let (url, job) = server(vec![
        (200, page(0, 4, vec![sure, rare, arena, legacy])),
        // Valeurs hors intervalle : toute la page est refusée, comme pour la borne basse.
        (200, page(0, 1, vec![with("win_rate_upper_bound", 120.0)])),
        (200, page(0, 1, vec![with("win_rate_delta", 101.0)])),
        (200, page(0, 1, vec![with("win_rate_delta", -100.5)])),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    let published: Vec<_> = report
        .builds
        .iter()
        .map(|b| (b.win_rate_upper_bound, b.win_rate_delta))
        .collect();
    assert_eq!(
        published,
        vec![
            (Some(58.9), Some(-2.5)),
            (None, None),
            (None, None),
            (None, None)
        ]
    );
    for _ in 0..3 {
        assert_eq!(
            client.builds(request()).await.unwrap_err(),
            BuildError::InvalidResponse
        );
    }
    job.await.unwrap();
}

fn with(field: &str, value: f64) -> Value {
    let mut row = variant("core", vec![6672, 3031, 3089]);
    row[field] = json!(value);
    row
}
