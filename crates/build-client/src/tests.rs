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

#[test]
fn accepte_les_paliers_cumules_de_l_api_et_refuse_les_autres_suffixes() {
    for rank in [
        "IRON_PLUS",
        "BRONZE_PLUS",
        "SILVER_PLUS",
        "GOLD_PLUS",
        "PLATINUM_PLUS",
        "EMERALD_PLUS",
        "DIAMOND_PLUS",
        "MASTER_PLUS",
    ] {
        let mut value = serde_json::to_value(request()).unwrap();
        value["rank"] = json!(rank);
        let req: BuildRequest = serde_json::from_value(value).unwrap();
        assert_eq!(req.validate(), Ok(()), "{rank}");
    }
    for rank in [
        "GRANDMASTER_PLUS",
        "CHALLENGER_PLUS",
        "ALL_PLUS",
        "emerald_plus",
    ] {
        let mut value = serde_json::to_value(request()).unwrap();
        value["rank"] = json!(rank);
        let req: BuildRequest = serde_json::from_value(value).unwrap();
        assert_eq!(req.validate(), Err(BuildError::InvalidRequest), "{rank}");
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
async fn transmet_le_taux_conditionnel_des_runes_et_le_masque_sous_le_seuil() {
    let mut slot = variant("rune_slot_1", vec![8005, 9111]);
    slot["conditional_rate"] = json!(72.0);
    let mut low = variant("rune_slot_2", vec![8005, 9104]);
    low["games"] = json!(2);
    low["wins"] = json!(1);
    low["conditional_rate"] = json!(4.0);
    let legacy = variant("rune_keystone", vec![8005]);
    let mut invalid = variant("rune_slot_3", vec![8005, 8014]);
    invalid["conditional_rate"] = json!(172.0);
    let (url, job) = server(vec![
        (200, page(0, 3, vec![slot, low, legacy])),
        (200, page(0, 1, vec![invalid])),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    let rates: Vec<_> = report.builds.iter().map(|b| b.conditional_rate).collect();
    assert_eq!(rates, vec![Some(72.0), None, None]);
    assert_eq!(
        client.builds(request()).await.unwrap_err(),
        BuildError::InvalidResponse
    );
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

#[tokio::test]
async fn transmet_borne_haute_et_fiabilite_et_masque_la_borne_sous_le_seuil() {
    // #91 : la fiabilité reste publiée sous min_games, la borne haute suit le taux.
    let mut core = variant("core", vec![6672, 3031, 3089]);
    core["win_rate_lower_bound"] = json!(41.2);
    core["win_rate_upper_bound"] = json!(58.8);
    core["reliability"] = json!("sufficient");
    let mut low = variant("starter", vec![1055, 2003]);
    low["games"] = json!(2);
    low["wins"] = json!(1);
    low["win_rate_lower_bound"] = json!(10.0);
    low["win_rate_upper_bound"] = json!(90.0);
    low["reliability"] = json!("low");
    // Arena : aucune performance publiable, la borne haute est écartée mais la fiabilité reste.
    let mut arena = variant("boots", vec![3006]);
    arena["performance_available"] = json!(false);
    arena["win_rate_upper_bound"] = json!(70.0);
    arena["reliability"] = json!("low");
    let legacy = variant("item", vec![1055]);
    let mut invalid = variant("core", vec![1, 2, 3]);
    invalid["win_rate_upper_bound"] = json!(120.0);
    let (url, job) = server(vec![
        (200, page(0, 4, vec![core, low, arena, legacy])),
        (200, page(0, 1, vec![invalid])),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    let upper: Vec<_> = report
        .builds
        .iter()
        .map(|b| b.win_rate_upper_bound)
        .collect();
    assert_eq!(upper, vec![Some(58.8), None, None, None]);
    let reliability: Vec<_> = report.builds.iter().map(|b| b.reliability).collect();
    assert_eq!(
        reliability,
        vec![
            Some(Reliability::Sufficient),
            Some(Reliability::Low),
            Some(Reliability::Low),
            None
        ]
    );
    assert_eq!(
        client.builds(request()).await.unwrap_err(),
        BuildError::InvalidResponse
    );
    job.await.unwrap();
}

#[tokio::test]
async fn transmet_le_placement_moyen_des_variantes_arena_hors_objets() {
    // Contrat avec le collecteur (#104) : en Arena, runes et sorts publient le placement
    // moyen et aucun taux de victoire ; sous le seuil, le placement est masqué.
    let mut arena = variant("runes", vec![8000, 8005]);
    arena["performance_available"] = json!(false);
    arena["wins"] = json!(null);
    arena["win_rate"] = json!(null);
    arena["placement_games"] = json!(120);
    arena["average_placement"] = json!(3.5);
    let mut low = arena.clone();
    low["selection"] = json!([8100, 8105]);
    low["games"] = json!(2);
    low["placement_games"] = json!(2);
    // Assez de parties jouées mais pas assez avec placement : masqué, comme au collecteur.
    let mut partial = arena.clone();
    partial["selection"] = json!([8200, 8205]);
    partial["placement_games"] = json!(2);
    partial["average_placement"] = json!(3.0);
    let legacy = variant("summoner_spells", vec![4, 14]);
    let (url, job) = server(vec![
        (200, page(0, 4, vec![arena, low, partial, legacy])),
        (
            200,
            page(
                0,
                1,
                vec![{
                    let mut invalid = variant("runes", vec![1]);
                    invalid["average_placement"] = json!(0.5);
                    invalid
                }],
            ),
        ),
    ])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let report = client.builds(request()).await.unwrap();
    let placements: Vec<_> = report
        .builds
        .iter()
        .map(|b| (b.placement_games, b.average_placement))
        .collect();
    assert_eq!(
        placements,
        vec![(120, Some(3.5)), (2, None), (2, None), (0, None)]
    );
    assert_eq!(report.builds[0].win_rate, None);
    assert_eq!(
        client.builds(request()).await.unwrap_err(),
        BuildError::InvalidResponse
    );
    job.await.unwrap();
}

fn with(field: &str, value: f64) -> Value {
    let mut row = variant("core", vec![6672, 3031, 3089]);
    row[field] = json!(value);
    row
}

#[tokio::test]
async fn transmet_population_et_couverture_avec_label_inconnu_tolerant() {
    let mut value = page(0, 0, vec![]);
    value["meta"]["population_label"] = json!("unknown_match_tier");
    value["meta"]["coverage"] = json!([{"patch":"16.19","platform_id":"EUW1","queue_id":420,"ranked_participations":100,"tier_participations":{"GOLD":8,"MASTER":92},"apex_share":0.92,"high_elo_biased":true}]);
    let (url, job) = server(vec![(200, value)]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let result = serde_json::to_value(client.builds(request()).await.unwrap()).unwrap();
    assert_eq!(result["meta"]["population_label"], "unknown_match_tier");
    assert_eq!(
        result["meta"]["coverage"][0]["tier_participations"]["MASTER"],
        92
    );
    assert_eq!(result["meta"]["coverage"][0]["high_elo_biased"], true);
    job.await.unwrap();
}

#[tokio::test]
async fn valide_effectifs_et_perimetre_sans_recalculer_le_biais() {
    let scope = json!({"patch":"16.19","platform_id":"EUW1","queue_id":420,"ranked_participations":100,"tier_participations":{"GOLD":100},"apex_share":0.0,"high_elo_biased":true});
    for (field, value, valid) in [
        ("tier_participations", json!({}), true),
        ("tier_participations", json!({"GOLD":99}), false),
        (
            "ranked_participations",
            json!(9_007_199_254_740_992_u64),
            false,
        ),
        ("queue_id", json!(440), false),
        ("apex_share", json!(1.01), false),
        ("apex_share", json!(null), true),
    ] {
        let mut row = scope.clone();
        row[field] = value;
        let mut value = page(0, 0, vec![]);
        value["meta"]["coverage"] = json!([row]);
        let (url, job) = server(vec![(200, value)]).await;
        let result = BuildClient::new(Some(url), Some("test-token".into()))
            .unwrap()
            .builds(request())
            .await;
        assert_eq!(result.is_ok(), valid, "{field}");
        job.await.unwrap();
    }
}

fn observation_details(value: &mut Value) {
    let key = json!({"champion_id":103,"patch":"16.19","platform_id":"EUW1","queue_id":420,"role":"MIDDLE","rank":"ALL"});
    let mut summary = key.clone();
    summary.as_object_mut().unwrap().extend(json!({"games":120,"wins":60,"losses":60,"population":200,"win_rate":50.0,"pick_rate":60.0}).as_object().unwrap().clone());
    let mut skill = key.clone();
    skill.as_object_mut().unwrap().extend(
        json!({"point":1,"slot":1,"games":120,"mean_timestamp_ms":45000.0})
            .as_object()
            .unwrap()
            .clone(),
    );
    let mut item = key;
    item.as_object_mut().unwrap().extend(
        json!({"event":"ITEM_PURCHASED","item_id":1001,"minute":3,"events":150})
            .as_object()
            .unwrap()
            .clone(),
    );
    value["summary"] = summary;
    value["skill_levels"] = json!([skill]);
    value["item_events"] = json!([item]);
    value["omitted_build_variants"] = json!(3);
    value["omitted_build_variants_by_category"] = json!([{"category":"runes","omitted":3}]);
    value["max_item_events"] = json!(2000);
    value["omitted_item_events"] = json!(7);
}
#[tokio::test]
async fn conserve_les_observations_une_seule_fois_apres_pagination() {
    let rows = (1..=200).map(|id| variant("item", vec![id])).collect();
    let mut first = page(0, 201, rows);
    observation_details(&mut first);
    let mut second = page(200, 201, vec![variant("item", vec![201])]);
    observation_details(&mut second);
    let (url, job) = server(vec![(200, first), (200, second)]).await;
    let result = BuildClient::new(Some(url), Some("test-token".into()))
        .unwrap()
        .builds(request())
        .await
        .unwrap();
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["summary"]["games"], 120);
    assert_eq!(result["skill_levels"].as_array().unwrap().len(), 1);
    assert_eq!(result["item_events"].as_array().unwrap().len(), 1);
    assert_eq!(result["omitted_build_variants"], 3);
    assert_eq!(result["omitted_item_events"], 7);
    job.await.unwrap();
}

#[tokio::test]
async fn refuse_changement_des_observations_et_les_lignes_hors_groupe() {
    let mut first = page(
        0,
        201,
        (1..=200).map(|id| variant("item", vec![id])).collect(),
    );
    observation_details(&mut first);
    let mut second = page(200, 201, vec![variant("item", vec![201])]);
    observation_details(&mut second);
    second["skill_levels"][0]["mean_timestamp_ms"] = json!(46000);
    let (url, job) = server(vec![(200, first), (200, second)]).await;
    assert!(matches!(
        BuildClient::new(Some(url), Some("test-token".into()))
            .unwrap()
            .builds(request())
            .await,
        Err(BuildError::ChangedSnapshot)
    ));
    job.await.unwrap();
    for (field, key, value) in [
        ("skill_levels", "champion_id", json!(86)),
        ("skill_levels", "slot", json!(5)),
        ("skill_levels", "mean_timestamp_ms", json!(-1)),
        ("item_events", "role", json!("TOP")),
        ("item_events", "events", json!(9_007_199_254_740_992_u64)),
    ] {
        let mut payload = page(0, 0, vec![]);
        observation_details(&mut payload);
        payload[field][0][key] = value;
        let (url, job) = server(vec![(200, payload)]).await;
        assert!(
            matches!(
                BuildClient::new(Some(url), Some("test-token".into()))
                    .unwrap()
                    .builds(request())
                    .await,
                Err(BuildError::InvalidResponse)
            ),
            "{field}.{key}"
        );
        job.await.unwrap();
    }
}
#[tokio::test]
async fn respecte_le_plafond_et_ne_presente_pas_un_ancien_compteur_global() {
    let mut payload = page(0, 0, vec![]);
    observation_details(&mut payload);
    payload["max_item_events"] = json!(0);
    let (url, job) = server(vec![(200, payload)]).await;
    assert!(matches!(
        BuildClient::new(Some(url), Some("test-token".into()))
            .unwrap()
            .builds(request())
            .await,
        Err(BuildError::InvalidResponse)
    ));
    job.await.unwrap();
    let mut legacy = page(0, 0, vec![]);
    legacy["omitted_build_variants"] = json!(35339);
    let (url, job) = server(vec![(200, legacy)]).await;
    let report = BuildClient::new(Some(url), Some("test-token".into()))
        .unwrap()
        .builds(request())
        .await
        .unwrap();
    assert_eq!(report.details.omitted_build_variants, None);
    job.await.unwrap();
}

#[tokio::test]
async fn accepte_des_omissions_partiellement_connues_sans_total_invente() {
    let mut value = page(0, 0, vec![]);
    observation_details(&mut value);
    value["omitted_build_variants"] = Value::Null;
    let (url, job) = server(vec![(200, value)]).await;
    let result = BuildClient::new(Some(url), Some("test-token".into()))
        .unwrap()
        .builds(request())
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().details.omitted_build_variants, None);
    job.await.unwrap();
}

#[tokio::test]
async fn manifeste_statique_borne_et_valide_les_versions() {
    let (url, job) = server(vec![(
        200,
        json!({"live_version":"16.20.1","versions":["16.20.1","16.19.1"],"catalogs":{}}),
    )])
    .await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    let result = client.static_versions().await.unwrap();
    assert_eq!(result.versions, vec!["16.20.1", "16.19.1"]);
    assert!(job.await.unwrap()[0].starts_with("GET /v1/static/manifest "));
    for value in [
        json!({"live_version":"bad","versions":[]}),
        json!({"live_version":"16.20.1","versions":["../secret"]}),
    ] {
        let (url, job) = server(vec![(200, value)]).await;
        let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
        assert!(matches!(
            client.static_versions().await,
            Err(BuildError::InvalidResponse)
        ));
        job.await.unwrap();
    }
}

#[tokio::test]
async fn catalogue_refuse_une_autre_release_et_un_manifeste_altere() {
    use olc_catalog_cache::*;
    let mut m = Manifest {
        schema_version: 1,
        version: "16.20.1".into(),
        normalizer_version: 3,
        snapshot_id: String::new(),
        files: std::collections::BTreeMap::from([(
            "test.json".into(),
            FileEntry {
                bytes: 2,
                sha256: digest(b"{}"),
                media_type: "application/json".into(),
            },
        )]),
    };
    m.snapshot_id = snapshot_id(&m).unwrap();
    let (url, job) = server(vec![(200, serde_json::to_value(&m).unwrap())]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    assert!(matches!(
        client.catalog_manifest("16.19.1", None).await,
        Err(BuildError::InvalidResponse)
    ));
    job.await.unwrap();
    m.files.get_mut("test.json").unwrap().bytes = 3;
    let (url, job) = server(vec![(200, serde_json::to_value(&m).unwrap())]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    assert!(matches!(
        client.catalog_manifest("16.20.1", None).await,
        Err(BuildError::InvalidResponse)
    ));
    job.await.unwrap();
}
#[tokio::test]
async fn catalogue_verifie_les_octets_et_le_chemin() {
    use olc_catalog_cache::*;
    let f = FileEntry {
        bytes: 2,
        sha256: digest(b"{}"),
        media_type: "application/json".into(),
    };
    let (url, job) = server(vec![(200, json!({}))]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    assert_eq!(
        client
            .catalog_file(&"a".repeat(64), "test.json", &f)
            .await
            .unwrap(),
        b"{}"
    );
    assert!(matches!(
        client
            .catalog_file(&"a".repeat(64), "../bad.json", &f)
            .await,
        Err(BuildError::InvalidRequest)
    ));
    job.await.unwrap();
}
#[tokio::test]
async fn reprise_du_catalogue_ne_redemande_pas_un_fichier_verifie() {
    use olc_catalog_cache::*;
    let dir = tempfile::tempdir().unwrap();
    let mut cache = Cache::open(dir.path()).unwrap();
    let a = FileEntry {
        bytes: 2,
        sha256: digest(b"{}"),
        media_type: "application/json".into(),
    };
    let b = FileEntry {
        bytes: 7,
        sha256: digest(b"{\"b\":1}"),
        media_type: "application/json".into(),
    };
    cache.put(&a, b"{}").unwrap();
    let mut m = Manifest {
        schema_version: 1,
        version: "16.20.1".into(),
        normalizer_version: 3,
        snapshot_id: String::new(),
        files: std::collections::BTreeMap::from([("a.json".into(), a), ("b.json".into(), b)]),
    };
    m.snapshot_id = snapshot_id(&m).unwrap();
    let (url, job) = server(vec![(200, json!({"b":1}))]).await;
    let client = BuildClient::new(Some(url), Some("test-token".into())).unwrap();
    client.download_catalog(&mut cache, &m).await.unwrap();
    assert!(cache.verify(&m).is_ok());
    assert!(cache.active().unwrap().is_none());
    let requests = job.await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("/b.json"));
}
