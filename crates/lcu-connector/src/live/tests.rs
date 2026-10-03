use super::*;
use serde_json::json;

fn payload() -> serde_json::Value {
    json!({"activePlayer":{"riotId":"Local#EUW","summonerName":"old-name"},
      "allPlayers":[{"riotId":"Enemy#EUW","rawChampionName":"game_character_displayname_Ahri","currentGold":9999},
       {"riotId":"Local#EUW","rawChampionName":"game_character_displayname_Bard","level":8,
        "scores":{"kills":1,"deaths":2,"assists":12,"creepScore":24},"items":[{"itemID":3009}]}],
      "gameData":{"gameTime":600.5,"gameMode":"CLASSIC","mapNumber":11},
      "events":{"Events":[{"EventID":0,"EventName":"GameStart","EventTime":0.0},
      {"EventID":1,"EventName":"ChampionKill","EventTime":5.0,"KillerName":"Enemy#EUW"}]}})
}
#[test]
fn ne_projette_que_le_joueur_local_et_les_evenements_publics_retenus() {
    let game = project(payload()).unwrap();
    assert_eq!(game.player.champion_key, "Bard");
    assert_eq!(game.player.assists, 12);
    assert_eq!(game.events.len(), 1);
    let serialized = serde_json::to_string(&game).unwrap();
    for forbidden in [
        "Enemy",
        "Local#",
        "riotId",
        "currentGold",
        "KillerName",
        "old-name",
    ] {
        assert!(!serialized.contains(forbidden));
    }
}
#[test]
fn refuse_les_donnees_incompletes_ou_une_identite_ambigue() {
    let mut data = payload();
    data["allPlayers"][1]["scores"]
        .as_object_mut()
        .unwrap()
        .remove("kills");
    assert!(project(data).is_err());
    let mut data = payload();
    let local = data["allPlayers"][1].clone();
    data["allPlayers"].as_array_mut().unwrap().push(local);
    assert!(project(data).is_err());
    assert!(project(json!({"error":"not ready"})).is_err());
}
#[test]
fn un_riot_id_present_ne_retombe_pas_sur_un_pseudo_different() {
    let mut data = payload();
    data["allPlayers"][1]["riotId"] = json!("Other#EUW");
    data["allPlayers"][1]["summonerName"] = json!("old-name");
    assert!(project(data).is_err());
}
#[test]
fn suit_une_partie_sans_reinitialiser_au_changement_de_phase() {
    let mut tracker = LiveTracker::default();
    let mut lcu = crate::LcuSession::default();
    lcu.apply(crate::LcuEvent::Connected { port: 123 });
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::GameStart,
    });
    tracker.observe(&lcu);
    let generation = tracker.snapshot.generation;
    tracker.accept(Ok(project(payload()).unwrap()));
    assert_eq!(tracker.snapshot.status, LiveStatus::Ready);
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::InProgress,
    });
    tracker.observe(&lcu);
    assert_eq!(tracker.snapshot.generation, generation);
    assert!(tracker.snapshot.game.is_some());
    tracker.accept(Err(LiveError::Unavailable));
    assert!(tracker.snapshot.game.is_none());
    tracker.accept(Ok(project(payload()).unwrap()));
    lcu.apply(crate::LcuEvent::Disconnected);
    tracker.observe(&lcu);
    assert_eq!(tracker.snapshot.status, LiveStatus::Idle);
    tracker.accept(Ok(project(payload()).unwrap()));
    assert!(tracker.snapshot.game.is_none());
}
#[test]
fn deduplique_les_evenements_et_detecte_une_nouvelle_horloge() {
    let mut tracker = LiveTracker {
        active: true,
        ..Default::default()
    };
    tracker.accept(Ok(project(payload()).unwrap()));
    tracker.accept(Ok(project(payload()).unwrap()));
    assert_eq!(tracker.snapshot.game.as_ref().unwrap().events.len(), 1);
    let before = tracker.snapshot.generation;
    let mut data = payload();
    data["gameData"]["gameTime"] = json!(0.0);
    tracker.accept(Ok(project(data).unwrap()));
    assert_eq!(tracker.snapshot.generation, before + 1);
}
fn draft_session() -> crate::LcuSession {
    let mut lcu = crate::LcuSession::default();
    lcu.apply(crate::LcuEvent::Connected { port: 1 });
    lcu.apply(crate::LcuEvent::AccountChanged {
        account: Some(crate::LcuAccount {
            platform: "EUW1".into(),
            game_name: "Private".into(),
            profile_icon_id: None,
            tag_line: "TAG".into(),
        }),
    });
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::ChampSelect,
    });
    let mut draft = crate::DraftSession::parse(
        serde_json::from_str(include_str!(
            "../../tests/fixtures/champ-select-public.json"
        ))
        .unwrap(),
    )
    .unwrap();
    draft.supported = true;
    draft.queue_id = Some(420);
    let local = draft.allies.iter_mut().find(|p| p.local).unwrap();
    local.champion_id = Some(432);
    local.locked = true;
    local.position = Some("UTILITY".into());
    lcu.apply(crate::LcuEvent::DraftChanged { draft: Some(draft) });
    lcu
}
#[test]
fn conserve_la_derniere_draft_reelle_a_l_entree_en_partie_meme_apres_lecture_absente() {
    let mut tracker = LiveTracker::default();
    let mut lcu = draft_session();
    tracker.observe(&lcu);
    lcu.apply(crate::LcuEvent::DraftChanged { draft: None });
    tracker.observe(&lcu);
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::GameStart,
    });
    tracker.observe(&lcu);
    let context = tracker.snapshot.context.as_ref().unwrap();
    assert_eq!(context.champion_id, 432);
    assert_eq!(context.role.as_deref(), Some("UTILITY"));
    assert_eq!(context.platform.as_deref(), Some("EUW1"));
}
#[test]
fn annulation_de_draft_et_demarrage_en_cours_de_partie_n_inventent_pas_un_role() {
    let mut tracker = LiveTracker::default();
    let mut lcu = draft_session();
    tracker.observe(&lcu);
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::Lobby,
    });
    tracker.observe(&lcu);
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::InProgress,
    });
    tracker.observe(&lcu);
    assert!(tracker.snapshot.context.is_none());
}
#[tokio::test]
async fn lit_le_transport_local_et_refuse_redirection_corps_trop_grand_ou_invalide() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    for (status, extra, body, expected) in [
        (200, "", payload().to_string(), None),
        (
            302,
            "location: http://127.0.0.1:1/never\r\n",
            "{}".into(),
            Some(LiveError::Unavailable),
        ),
        (200, "", "not json".into(), Some(LiveError::Invalid)),
        (
            200,
            "content-length: 5000000\r\n",
            "".into(),
            Some(LiveError::Invalid),
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/liveclientdata/allgamedata",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let n = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]).to_lowercase();
            assert!(request.starts_with("get /liveclientdata/allgamedata http/1.1"));
            assert!(!request.contains("authorization"));
            let size = if extra.contains("content-length") {
                String::new()
            } else {
                format!("content-length: {}\r\n", body.len())
            };
            let response =
                format!("HTTP/1.1 {status} Test\r\n{extra}{size}connection: close\r\n\r\n{body}");
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let actual = LiveClient::new().unwrap().read_url(&url).await;
        assert_eq!(actual.err(), expected);
        server.await.unwrap();
    }
}
#[tokio::test]
async fn une_lecture_bloquee_a_un_delai_fini() {
    use tokio::{io::AsyncReadExt, net::TcpListener};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/liveclientdata/allgamedata",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut b = [0; 4096];
        let _ = stream.read(&mut b).await;
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    let result = tokio::time::timeout(
        Duration::from_secs(4),
        LiveClient::new().unwrap().read_url(&url),
    )
    .await
    .unwrap();
    assert_eq!(result.err(), Some(LiveError::Unavailable));
    server.abort();
}
#[test]
fn le_poste_assigne_reste_prioritaire_en_personnalisee() {
    let mut lcu = draft_session();
    lcu.draft.as_mut().unwrap().custom_game = true;
    let mut tracker = LiveTracker::default();
    tracker.observe(&lcu);
    tracker.custom_role(Some("TOP"));
    assert_eq!(
        tracker.pending.as_ref().unwrap().role.as_deref(),
        Some("UTILITY")
    );
}
#[tokio::test]
async fn le_canal_regroupe_la_draft_et_game_start_sans_perdre_le_champion() {
    let (tx, rx) = watch::channel(LiveInput::default());
    let mut lcu = draft_session();
    tx.send_modify(|i| i.set_session(&lcu));
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::GameStart,
    });
    tx.send_modify(|i| i.set_session(&lcu));
    let (out, mut receive) = mpsc::channel(4);
    let task = tokio::spawn(super::watch_with_reader(
        rx,
        out,
        std::future::pending::<Result<LiveGame, LiveError>>,
    ));
    let live = receive.recv().await.unwrap();
    task.abort();
    assert_eq!(live.status, LiveStatus::Waiting);
    assert_eq!(live.context.as_ref().unwrap().champion_id, 432);
}

#[test]
fn publie_chaque_cs_recu_sans_arrondir_ni_attendre_un_palier() {
    let mut tracker = LiveTracker {
        active: true,
        ..Default::default()
    };
    for (index, cs) in [9, 10, 11, 19, 20].into_iter().enumerate() {
        let mut data = payload();
        data["allPlayers"][1]["scores"]["creepScore"] = json!(cs);
        data["gameData"]["gameTime"] = json!(100.0 + index as f64);
        tracker.accept(project(data));
        assert_eq!(
            tracker.snapshot.game.as_ref().unwrap().player.creep_score,
            cs
        );
        assert_eq!(tracker.snapshot.revision, index as u32 + 1);
    }
}

type PendingRead = tokio::sync::oneshot::Sender<Result<LiveGame, LiveError>>;

fn start_controlled_watch() -> (
    watch::Sender<LiveInput>,
    mpsc::Receiver<LiveSession>,
    mpsc::UnboundedReceiver<PendingRead>,
    tokio::task::JoinHandle<()>,
) {
    let mut lcu = draft_session();
    let mut input = LiveInput::default();
    input.set_session(&lcu);
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::InProgress,
    });
    input.set_session(&lcu);
    let (send, receive) = watch::channel(input);
    let (output, snapshots) = mpsc::channel(16);
    let (reads, pending) = mpsc::unbounded_channel();
    // Seule la lecture externe est remplacée ; cadence et transitions restent réelles.
    let task = tokio::spawn(watch_with_reader(receive, output, move || {
        let (reply, result) = tokio::sync::oneshot::channel();
        reads.send(reply).unwrap();
        async move { result.await.unwrap() }
    }));
    (send, snapshots, pending, task)
}

#[tokio::test(start_paused = true)]
async fn annule_la_lecture_en_vol_a_la_sortie_et_rejette_sa_reponse_tardive() {
    let (input, mut snapshots, mut reads, task) = start_controlled_watch();
    let initial = snapshots.recv().await.unwrap();
    assert_eq!(initial.status, LiveStatus::Waiting);
    let pending = reads.recv().await.unwrap();
    input.send_modify(|i| i.set_session(&LcuSession::default()));
    let idle = tokio::time::timeout(Duration::from_secs(1), snapshots.recv())
        .await
        .expect("la sortie ne doit pas attendre la réponse réseau")
        .unwrap();
    assert_eq!(idle.status, LiveStatus::Idle);
    assert!(idle.game.is_none());
    assert!(idle.context.is_none());
    assert!(idle.generation > initial.generation);
    assert!(pending.send(project(payload())).is_err());
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::task::yield_now().await;
    assert!(snapshots.try_recv().is_err());
    assert!(reads.try_recv().is_err());
    drop(input);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn efface_les_donnees_sur_coupure_et_reprend_sans_requete_concurrente() {
    let (input, mut snapshots, mut reads, task) = start_controlled_watch();
    let initial = snapshots.recv().await.unwrap();
    let pending = reads.recv().await.unwrap();
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::task::yield_now().await;
    assert!(reads.try_recv().is_err());
    pending.send(project(payload())).unwrap();
    let ready = snapshots.recv().await.unwrap();
    assert_eq!(ready.game.as_ref().unwrap().player.creep_score, 24);
    assert_eq!(ready.generation, initial.generation);
    tokio::time::advance(Duration::from_millis(999)).await;
    tokio::task::yield_now().await;
    assert!(reads.try_recv().is_err());
    tokio::time::advance(Duration::from_millis(1)).await;
    reads
        .recv()
        .await
        .unwrap()
        .send(Err(LiveError::Unavailable))
        .unwrap();
    let unavailable = snapshots.recv().await.unwrap();
    assert_eq!(unavailable.status, LiveStatus::Unavailable);
    assert!(unavailable.game.is_none());
    assert_eq!(unavailable.generation, ready.generation);
    let mut recovered = payload();
    recovered["gameData"]["gameTime"] = json!(602.0);
    recovered["allPlayers"][1]["scores"]["creepScore"] = json!(25);
    reads
        .recv()
        .await
        .unwrap()
        .send(project(recovered))
        .unwrap();
    let resumed = snapshots.recv().await.unwrap();
    assert_eq!(resumed.status, LiveStatus::Ready);
    assert_eq!(resumed.game.as_ref().unwrap().player.creep_score, 25);
    assert!(resumed.revision > unavailable.revision);
    assert_eq!(resumed.generation, ready.generation);
    drop(input);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn deux_parties_coalescees_effacent_le_contexte_et_annulent_l_ancienne_lecture() {
    let (input, mut snapshots, mut reads, task) = start_controlled_watch();
    snapshots.recv().await.unwrap();
    reads
        .recv()
        .await
        .unwrap()
        .send(project(payload()))
        .unwrap();
    let first = snapshots.recv().await.unwrap();
    assert!(first.context.is_some());
    let pending = reads.recv().await.unwrap();
    let mut next = LcuSession::default();
    next.apply(crate::LcuEvent::Connected { port: 123 });
    next.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::InProgress,
    });
    input.send_modify(|i| {
        i.set_session(&LcuSession::default());
        i.set_session(&next);
    });
    let waiting = tokio::time::timeout(Duration::from_secs(1), snapshots.recv())
        .await
        .expect("les transitions regroupées doivent annuler la lecture")
        .unwrap();
    assert_eq!(waiting.status, LiveStatus::Waiting);
    assert!(waiting.game.is_none());
    assert!(waiting.context.is_none());
    assert!(waiting.generation > first.generation);
    assert!(pending.send(project(payload())).is_err());
    let mut second = payload();
    second["gameData"]["gameTime"] = json!(2.0);
    second["allPlayers"][1]["rawChampionName"] = json!("game_character_displayname_Mel");
    second["allPlayers"][1]["scores"]["creepScore"] = json!(0);
    reads.recv().await.unwrap().send(project(second)).unwrap();
    let ready = snapshots.recv().await.unwrap();
    let game = ready.game.unwrap();
    assert_eq!(game.player.champion_key, "Mel");
    assert_eq!(game.game_time, 2.0);
    assert_eq!(game.player.creep_score, 0);
    assert_eq!(ready.generation, waiting.generation);
    assert!(ready.context.is_none());
    assert!(ready.revision > first.revision);
    drop(input);
    task.await.unwrap();
}

#[test]
fn un_recul_d_horloge_efface_le_contexte_de_la_partie_precedente() {
    let mut tracker = LiveTracker::default();
    let mut lcu = draft_session();
    tracker.observe(&lcu);
    lcu.apply(crate::LcuEvent::PhaseChanged {
        phase: crate::GameflowPhase::InProgress,
    });
    tracker.observe(&lcu);
    tracker.accept(project(payload()));
    assert!(tracker.snapshot.context.is_some());
    let generation = tracker.snapshot.generation;
    let mut second = payload();
    second["gameData"]["gameTime"] = json!(0.5);
    second["allPlayers"][1]["scores"]["creepScore"] = json!(0);
    tracker.accept(project(second));
    assert!(tracker.snapshot.context.is_none());
    assert!(tracker.snapshot.generation > generation);
    assert_eq!(tracker.snapshot.game.as_ref().unwrap().game_time, 0.5);
    assert_eq!(
        tracker.snapshot.game.as_ref().unwrap().player.creep_score,
        0
    );
}

#[test]
fn projette_la_capture_reelle_macos_sans_identite_et_sans_alterer_les_valeurs() {
    let source: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/live-client-allgamedata-macos-2026-10-03.json"
    ))
    .unwrap();
    let game = project(source).unwrap();
    assert_eq!(game.player.champion_key, "Mel");
    assert_eq!(game.player.level, 4);
    assert_eq!(
        (game.player.kills, game.player.deaths, game.player.assists),
        (0, 0, 0)
    );
    assert_eq!(game.player.creep_score, 20);
    assert_eq!(game.player.items, [1056, 2003, 2010, 3340]);
    assert_eq!(game.game_time, 192.2555694580078);
    assert_eq!(game.map_number, 11);
    assert_eq!(game.game_mode, "CLASSIC");
    assert_eq!(
        game.events
            .iter()
            .map(|event| event.name.as_str())
            .collect::<Vec<_>>(),
        ["GameStart", "MinionsSpawning"]
    );
    let projected = serde_json::to_string(&game).unwrap();
    for identity in ["fixture-local", "riotId", "summonerName", "puuid"] {
        assert!(!projected.contains(identity));
    }
}
