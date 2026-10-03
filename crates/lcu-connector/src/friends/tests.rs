use super::*;
use crate::account::{ACCOUNT_ENDPOINT, REGION_ENDPOINT};
use crate::test_support::{mock_client, ExpectedRequest};
use serde_json::json;

fn sample() -> Value {
    json!({
        "id":"fixture-chat-id", "puuid":"fixture-puuid", "pid":"fixture-pid",
        "summonerId":123, "note":"note privée fictive", "statusMessage":"message fictif",
        "gameName":"Alpha Player", "gameTag":"TAG", "name":"Legacy Name",
        "platformId":"EUW1", "icon":17, "availability":"chat", "product":"league_of_legends",
        "lol":{"gameStatus":"inGame", "championId":"1"}
    })
}

fn expected_account() -> LcuAccount {
    LcuAccount {
        platform: "EUW1".into(),
        game_name: "Owner".into(),
        profile_icon_id: None,
        tag_line: "TAG".into(),
    }
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

fn account_reads(name: &str) -> Vec<ExpectedRequest> {
    let summoner = json!({"gameName":name, "tagLine":"TAG"});
    vec![
        get(ACCOUNT_ENDPOINT, summoner.clone()),
        get(REGION_ENDPOINT, json!({"region":"EUW"})),
        get(ACCOUNT_ENDPOINT, summoner),
    ]
}

fn ready_reads(friends: Value) -> Vec<ExpectedRequest> {
    let mut requests = account_reads("Owner");
    requests.push(get(SESSION_ENDPOINT, json!({"sessionState":"loaded"})));
    requests.push(get(FRIENDS_ENDPOINT, friends));
    requests.extend(account_reads("Owner"));
    requests.push(get(SESSION_ENDPOINT, json!({"sessionState":"loaded"})));
    requests
}

#[test]
fn projette_identite_presence_et_icone_sans_identifiant_technique() {
    let friends = parse_friends(&json!([sample()])).unwrap();
    assert_eq!(
        serde_json::to_value(&friends).unwrap(),
        json!([{
            "name":"Alpha Player", "game_name":"Alpha Player", "tag_line":"TAG",
            "platform":"EUW1", "icon_id":17, "presence":"online"
        }])
    );
}

#[test]
fn un_riot_id_incomplet_reste_affichable_sans_devenir_recherchable() {
    let friends = parse_friends(&json!([
        {"name":"Legacy Name", "gameName":"", "gameTag":"", "availability":"offline"},
        {"gameName":"New Player", "gameTag":"", "availability":"away"}
    ]))
    .unwrap();
    assert_eq!(friends[0].name, "Legacy Name");
    assert_eq!(friends[1].name, "New Player");
    assert!(friends.iter().all(|friend| friend.game_name.is_none()
        && friend.tag_line.is_none()
        && friend.platform.is_none()));
}

#[test]
fn ne_deduit_jamais_la_plateforme_du_tag_ou_du_compte_connecte() {
    let friends = parse_friends(&json!([
        {"gameName":"First", "gameTag":"EUW1", "platformId":"NA1"},
        {"gameName":"Second", "gameTag":"EUW1", "platformId":"PBE"},
        {"gameName":"Third", "gameTag":"EUW1"}
    ]))
    .unwrap();
    assert_eq!(friends[0].platform.as_deref(), Some("NA1"));
    assert_eq!(friends[1].platform, None);
    assert_eq!(friends[2].platform, None);
}

#[test]
fn les_presences_observees_sont_normalisees_et_les_autres_restent_inconnues() {
    for (availability, expected) in [
        ("chat", FriendPresence::Online),
        ("away", FriendPresence::Away),
        ("dnd", FriendPresence::Busy),
        ("offline", FriendPresence::Offline),
        ("unrecognized", FriendPresence::Unknown),
        ("", FriendPresence::Unknown),
    ] {
        let value = json!([{"name":"Friend", "availability":availability}]);
        assert_eq!(parse_friends(&value).unwrap()[0].presence, expected);
    }
}

#[test]
fn ignore_les_entrees_sans_nom_utilisable_et_ne_confond_pas_corruption_et_vide() {
    assert_eq!(parse_friends(&json!([])), Some(vec![]));
    assert_eq!(parse_friends(&json!({"errorCode":"unavailable"})), None);
    assert_eq!(parse_friends(&json!([null,{}, {"name":"bad\nname"}])), None);
    let friends = parse_friends(
        &json!([null, {}, {"name":"Friend", "icon":-1}, {"name":"Other", "icon":4294967296u64}]),
    )
    .unwrap();
    assert_eq!(friends.len(), 2);
    assert!(friends.iter().all(|friend| friend.icon_id.is_none()));
}

#[tokio::test]
async fn publie_la_liste_uniquement_pour_le_compte_attendu_et_un_chat_charge() {
    let (client, server) = mock_client(ready_reads(json!([sample()]))).await;
    let result = read_friends(&client, &expected_account()).await;
    assert_eq!(result.status, FriendsStatus::Ready);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].name, "Alpha Player");
    server.await.unwrap();
}

#[tokio::test]
async fn une_liste_vide_chargee_est_un_resultat_valide() {
    let (client, server) = mock_client(ready_reads(json!([]))).await;
    let result = read_friends(&client, &expected_account()).await;
    assert_eq!(
        result,
        FriendsSnapshot {
            status: FriendsStatus::Ready,
            items: vec![]
        }
    );
    server.await.unwrap();
}

#[tokio::test]
async fn une_session_sociale_en_initialisation_ne_simule_pas_zero_ami() {
    for state in ["initializing", "connected"] {
        let mut requests = account_reads("Owner");
        requests.push(get(SESSION_ENDPOINT, json!({"sessionState":state})));
        let (client, server) = mock_client(requests).await;
        let result = read_friends(&client, &expected_account()).await;
        assert_eq!(
            result,
            FriendsSnapshot {
                status: FriendsStatus::Loading,
                items: vec![]
            }
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn un_compte_different_avant_la_lecture_empeche_de_lire_ses_amis() {
    let (client, server) = mock_client(account_reads("Another Owner")).await;
    assert_eq!(
        read_friends(&client, &expected_account()).await.status,
        FriendsStatus::Unavailable
    );
    server.await.unwrap();
}

#[tokio::test]
async fn un_changement_de_compte_pendant_la_lecture_invalide_toute_la_liste() {
    let mut requests = account_reads("Owner");
    requests.push(get(SESSION_ENDPOINT, json!({"sessionState":"loaded"})));
    requests.push(get(FRIENDS_ENDPOINT, json!([sample()])));
    requests.extend(account_reads("Another Owner"));
    let (client, server) = mock_client(requests).await;
    assert_eq!(
        read_friends(&client, &expected_account()).await,
        FriendsSnapshot {
            status: FriendsStatus::Unavailable,
            items: vec![]
        }
    );
    server.await.unwrap();
}

#[tokio::test]
async fn une_deconnexion_sociale_pendant_la_lecture_efface_les_amis_lus() {
    let mut requests = ready_reads(json!([sample()]));
    requests.last_mut().unwrap().response = json!({"sessionState":"disconnected"});
    let (client, server) = mock_client(requests).await;
    assert_eq!(
        read_friends(&client, &expected_account()).await,
        FriendsSnapshot {
            status: FriendsStatus::Unavailable,
            items: vec![]
        }
    );
    server.await.unwrap();
}

#[tokio::test]
async fn les_erreurs_http_ne_deviennent_pas_une_liste_vide_prete() {
    for failed_index in [0, 3, 4, 5, 8] {
        let mut requests = ready_reads(json!([sample()]));
        requests.truncate(failed_index + 1);
        requests[failed_index].status = 503;
        let (client, server) = mock_client(requests).await;
        assert_eq!(
            read_friends(&client, &expected_account()).await,
            FriendsSnapshot {
                status: FriendsStatus::Unavailable,
                items: vec![]
            }
        );
        server.await.unwrap();
    }
}
