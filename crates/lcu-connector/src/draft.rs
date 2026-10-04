//! Lecture seule de `/lol-champ-select/v1/session`.
//! Schéma : https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-champ-select/global/default/rcp-fe-lol-champ-select.js
//! Dump Riot 16.19 du 29/09/2026 : team 1 = bleu, 2 = rouge.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DRAFT_ENDPOINT: &str = "/lol-champ-select/v1/session";
pub(crate) const FLOW_ENDPOINT: &str = "/lol-gameflow/v1/session";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DraftMode {
    StandardRift(Option<u32>),
    CustomRift(Option<u32>),
    Unsupported,
}
impl DraftMode {
    /// Métadonnées documentées du gameflow ; aucun identifiant de joueur n'est conservé.
    pub(crate) fn from_flow(flow: &Value) -> Self {
        if flow["phase"] != "ChampSelect" {
            return Self::Unsupported;
        }
        let data = &flow["gameData"];
        if data["isCustomGame"] == true {
            return if flow["map"]["id"] == 11 && flow["map"]["gameMode"] == "CLASSIC" {
                Self::CustomRift(
                    data["queue"]["id"]
                        .as_u64()
                        .and_then(|id| u32::try_from(id).ok()),
                )
            } else {
                Self::Unsupported
            };
        }
        let queue = &data["queue"];
        if queue["mapId"] == 11
            && queue["gameMode"] == "CLASSIC"
            && matches!(queue["id"].as_u64(), Some(400 | 420 | 440))
        {
            Self::StandardRift(queue["id"].as_u64().map(|id| id as u32))
        } else {
            Self::Unsupported
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Blue,
    Red,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPlayer {
    pub cell_id: i32,
    pub champion_id: Option<u32>,
    pub locked: bool,
    pub local: bool,
    pub position: Option<String>,
    pub acting: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftTimer {
    pub remaining_ms: u64,
    pub observed_at_ms: u64,
}

/// Projection publique minimale : jamais d'identité, de chat ni de secrets LCU.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSession {
    /// Identité de partie publique, en chaîne pour préserver les entiers 64 bits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<u32>,
    #[serde(rename = "customGame")]
    pub custom_game: bool,
    pub supported: bool,
    pub ally_side: Option<Side>,
    pub allies: Vec<DraftPlayer>,
    pub enemies: Vec<DraftPlayer>,
    pub ally_bans: Vec<u32>,
    pub enemy_bans: Vec<u32>,
    pub timer: Option<DraftTimer>,
    pub local_spells: Option<[u32; 2]>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSession {
    #[serde(default)]
    game_id: Option<u64>,
    my_team: Vec<RawPlayer>,
    their_team: Vec<RawPlayer>,
    actions: Vec<Vec<RawAction>>,
    local_player_cell_id: i32,
    #[serde(default)]
    bans: RawBans,
    #[serde(default)]
    timer: Option<RawTimer>,
    #[serde(default)]
    bench_enabled: bool,
    #[serde(default)]
    allow_rerolling: bool,
    #[serde(default)]
    allow_duplicate_picks: bool,
    #[serde(default)]
    is_spectating: bool,
    #[serde(default)]
    skip_champion_select: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPlayer {
    cell_id: i32,
    #[serde(default)]
    champion_id: u32,
    #[serde(default)]
    champion_pick_intent: u32,
    #[serde(default)]
    assigned_position: String,
    #[serde(default)]
    spell1_id: u32,
    #[serde(default)]
    spell2_id: u32,
    #[serde(default)]
    team: i32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAction {
    actor_cell_id: i32,
    #[serde(rename = "type")]
    kind: String,
    completed: bool,
    #[serde(default)]
    is_in_progress: bool,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBans {
    #[serde(default)]
    my_team_bans: Vec<i32>,
    #[serde(default)]
    their_team_bans: Vec<i32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTimer {
    adjusted_time_left_in_phase: i64,
    internal_now_in_epoch_ms: i64,
    #[serde(default)]
    is_infinite: bool,
}
impl DraftSession {
    pub fn parse(value: Value) -> Option<Self> {
        Self::parse_for_mode(value, DraftMode::StandardRift(None))
    }
    pub(crate) fn parse_for_mode(value: Value, mode: DraftMode) -> Option<Self> {
        let raw: RawSession = serde_json::from_value(value).ok()?;
        let supported = !raw.bench_enabled
            && !raw.allow_rerolling
            && !raw.allow_duplicate_picks
            && !raw.is_spectating
            && !raw.skip_champion_select
            && match mode {
                DraftMode::StandardRift(_) => raw.my_team.len() == 5 && raw.their_team.len() == 5,
                DraftMode::CustomRift(_) => {
                    !raw.my_team.is_empty() && raw.my_team.len() <= 5 && raw.their_team.len() <= 5
                }
                DraftMode::Unsupported => false,
            };
        let ally_side = raw
            .my_team
            .iter()
            .find(|p| p.cell_id == raw.local_player_cell_id)
            .and_then(|p| match p.team {
                1 => Some(Side::Blue),
                2 => Some(Side::Red),
                _ => None,
            });
        let players = |team: &[RawPlayer], enemy: bool| {
            team.iter()
                .take(5)
                .map(|p| {
                    let locked = p.champion_id > 0
                        && raw.actions.iter().flatten().any(|a| {
                            a.kind == "pick" && a.actor_cell_id == p.cell_id && a.completed
                        });
                    // Même si l'API divulgue une intention adverse, elle ne quitte pas Rust.
                    let id = if enemy && !locked {
                        0
                    } else if p.champion_id > 0 {
                        p.champion_id
                    } else {
                        p.champion_pick_intent
                    };
                    let position = (!enemy
                        && matches!(
                            p.assigned_position.as_str(),
                            "top" | "jungle" | "middle" | "bottom" | "utility"
                        ))
                    .then(|| p.assigned_position.clone());
                    DraftPlayer {
                        cell_id: p.cell_id,
                        champion_id: (id > 0).then_some(id),
                        locked,
                        local: !enemy && p.cell_id == raw.local_player_cell_id,
                        position,
                        acting: raw.actions.iter().flatten().any(|a| {
                            a.actor_cell_id == p.cell_id
                                && !a.completed
                                && a.is_in_progress
                                && matches!(a.kind.as_str(), "pick" | "ban")
                        }),
                    }
                })
                .collect()
        };
        let bans = |values: Vec<i32>| {
            values
                .into_iter()
                .filter(|id| *id > 0)
                .take(5)
                .map(|id| id as u32)
                .collect()
        };
        let mut local = raw
            .my_team
            .iter()
            .filter(|p| p.cell_id == raw.local_player_cell_id);
        let local_spells = local
            .next()
            .and_then(|p| {
                (p.spell1_id > 0 && p.spell2_id > 0 && p.spell1_id != p.spell2_id)
                    .then_some([p.spell1_id, p.spell2_id])
            })
            .filter(|_| local.next().is_none());
        Some(Self {
            game_id: raw.game_id.filter(|id| *id > 0).map(|id| id.to_string()),
            queue_id: match mode {
                DraftMode::StandardRift(queue) | DraftMode::CustomRift(queue) => queue,
                _ => None,
            },
            custom_game: matches!(mode, DraftMode::CustomRift(_)),
            local_spells,
            supported,
            ally_side,
            allies: players(&raw.my_team, false),
            enemies: players(&raw.their_team, true),
            ally_bans: bans(raw.bans.my_team_bans),
            enemy_bans: bans(raw.bans.their_team_bans),
            timer: raw
                .timer
                .filter(|t| !t.is_infinite && t.internal_now_in_epoch_ms > 0)
                .map(|t| DraftTimer {
                    remaining_ms: t.adjusted_time_left_in_phase.clamp(0, 3600000) as u64,
                    observed_at_ms: t.internal_now_in_epoch_ms as u64,
                }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/champ-select-public.json")).unwrap()
    }
    #[test]
    fn projette_la_partie_sans_perte_de_precision_et_la_file_verifiee() {
        let mut raw = fixture();
        raw["gameId"] = json!(9007199254740993_u64);
        let flow = json!({"phase":"ChampSelect","gameData":{"queue":{"id":420,"mapId":11,"gameMode":"CLASSIC"}}});
        let draft = DraftSession::parse_for_mode(raw, DraftMode::from_flow(&flow)).unwrap();
        let public = serde_json::to_value(draft).unwrap();
        assert_eq!(public["gameId"], "9007199254740993");
        assert_eq!(public["queueId"], 420);
        let unknown = serde_json::to_value(DraftSession::parse(fixture()).unwrap()).unwrap();
        assert!(unknown.get("gameId").is_none());
        assert!(unknown.get("queueId").is_none());
    }
    #[test]
    fn personnalisee_identifiee_accepte_equipes_incompletes_sans_debloquer_autres_modes() {
        let raw = fixture();
        let custom = json!({"phase":"ChampSelect","gameData":{"isCustomGame":true,"queue":{"id":0}},"map":{"id":11,"gameMode":"CLASSIC"}});
        let mode = DraftMode::from_flow(&custom);
        assert_eq!(mode, DraftMode::CustomRift(Some(0)));
        assert!(
            DraftSession::parse_for_mode(raw.clone(), mode)
                .unwrap()
                .supported
        );
        for map in [0, 12, 30] {
            let mut other = custom.clone();
            other["map"]["id"] = json!(map);
            assert!(
                !DraftSession::parse_for_mode(raw.clone(), DraftMode::from_flow(&other))
                    .unwrap()
                    .supported
            );
        }
        let mut other = custom.clone();
        other["gameData"]["isCustomGame"] = json!(false);
        assert_eq!(DraftMode::from_flow(&other), DraftMode::Unsupported);
        let mut other = custom.clone();
        other["map"]["gameMode"] = json!("PRACTICETOOL");
        assert_eq!(DraftMode::from_flow(&other), DraftMode::Unsupported);
        let mut alternate = raw.clone();
        alternate["benchEnabled"] = json!(true);
        assert!(
            !DraftSession::parse_for_mode(alternate, mode)
                .unwrap()
                .supported
        );
        let mut oversize = raw.clone();
        oversize["myTeam"] = json!((0..6)
            .map(|cell| json!({"cellId":cell}))
            .collect::<Vec<_>>());
        assert!(
            !DraftSession::parse_for_mode(oversize, mode)
                .unwrap()
                .supported
        );
        assert!(
            !DraftSession::parse_for_mode(raw, DraftMode::Unsupported)
                .unwrap()
                .supported
        );
    }
    #[test]
    fn exporte_seulement_les_deux_sorts_du_joueur_local() {
        let mut v = fixture();
        v["localPlayerCellId"] = json!(0);
        v["myTeam"][0]["cellId"] = json!(0);
        v["myTeam"][0]["spell1Id"] = json!(4);
        v["myTeam"][0]["spell2Id"] = json!(14);
        let d = DraftSession::parse(v.clone()).unwrap();
        assert_eq!(d.local_spells, Some([4, 14]));
        let out = serde_json::to_value(d).unwrap();
        assert_eq!(out["localSpells"], json!([4, 14]));
        assert!(out["allies"][0].get("spell1Id").is_none());
        assert!(out["enemies"][0].get("spell1Id").is_none());
        v["myTeam"][0]["spell2Id"] = json!(0);
        assert_eq!(DraftSession::parse(v).unwrap().local_spells, None);
    }
    #[test]
    fn distingue_intention_et_verrouillage_sans_exporter_d_identite() {
        let mut v = fixture();
        v["myTeam"][0]["championId"] = json!(0);
        v["myTeam"][0]["championPickIntent"] = json!(103);
        let d = DraftSession::parse(v.clone()).unwrap();
        assert_eq!(d.allies[0].champion_id, Some(103));
        assert!(!d.allies[0].locked);
        v["myTeam"][0]["championId"] = json!(63);
        v["actions"][3][0]["completed"] = json!(true);
        let d = DraftSession::parse(v).unwrap();
        assert!(d.allies[0].locked);
        assert_eq!(d.allies[0].champion_id, Some(63));
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("summoner"));
        assert!(!json.contains("chat"));
    }
    #[test]
    fn aucune_identite_des_joueurs_ne_sort_de_la_projection() {
        // Anonymat de la sélection (#30) : même si le client divulgue des identités, ni la
        // projection ni ses clés ne les reprennent. Les marqueurs sont des canaris synthétiques.
        let mut v = fixture();
        let identity = json!({
            "puuid": "CANARY-PUUID", "gameName": "CANARY-GAME", "tagLine": "CANARY-TAG",
            "summonerName": "CANARY-SUMMONER", "summonerId": 424242, "obfuscatedPuuid": "CANARY-OBF",
            "obfuscatedSummonerId": 434343, "nameVisibilityType": "VISIBLE",
            "chatRoomName": "CANARY-CHAT", "spell1Id": 4, "spell2Id": 14, "skinId": 7
        });
        for team in ["myTeam", "theirTeam"] {
            v[team] = json!((0..5)
                .map(|cell| {
                    let mut player = identity.clone();
                    player["cellId"] = json!(cell + if team == "myTeam" { 0 } else { 5 });
                    player["team"] = json!(if team == "myTeam" { 1 } else { 2 });
                    player
                })
                .collect::<Vec<_>>());
        }
        v["chatDetails"] =
            json!({"chatRoomName": "CANARY-CHAT", "mucJwtDto": {"jwt": "CANARY-JWT"}});
        let draft = DraftSession::parse(v).unwrap();
        let value = serde_json::to_value(&draft).unwrap();
        let text = value.to_string();
        for forbidden in [
            "CANARY", "puuid", "gameName", "tagLine", "summoner", "chat", "424242",
        ] {
            assert!(
                !text.to_lowercase().contains(&forbidden.to_lowercase()),
                "{forbidden} sorti : {text}"
            );
        }
        // Liste blanche des clés : tout nouveau champ doit être ajouté ici après revue de conformité.
        let players = ["allies", "enemies"].map(|k| value[k][0].as_object().unwrap().clone());
        for player in players {
            let mut keys: Vec<_> = player.keys().map(String::as_str).collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                [
                    "acting",
                    "cellId",
                    "championId",
                    "local",
                    "locked",
                    "position"
                ]
            );
        }
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "allies",
                "allyBans",
                "allySide",
                "customGame",
                "enemies",
                "enemyBans",
                "localSpells",
                "supported",
                "timer"
            ]
        );
    }
    #[test]
    fn utilise_le_cote_du_joueur_local_pas_son_index() {
        let mut v = fixture();
        v["myTeam"][0]["team"] = json!(2);
        v["myTeam"][0]["cellId"] = json!(7);
        v["localPlayerCellId"] = json!(7);
        assert_eq!(
            DraftSession::parse(v.clone()).unwrap().ally_side,
            Some(Side::Red)
        );
        v["myTeam"][0]["team"] = json!(0);
        assert_eq!(DraftSession::parse(v).unwrap().ally_side, None);
    }
    #[test]
    fn accepte_une_draft_cinq_contre_cinq_et_masque_les_postes_adverses() {
        let mut v = fixture();
        v["myTeam"] = json!((0..5)
            .map(|cell| json!({"cellId":cell,"team":2,"assignedPosition":"middle"}))
            .collect::<Vec<_>>());
        v["theirTeam"] = json!((5..10)
            .map(|cell| json!({"cellId":cell,"team":1,"assignedPosition":"middle"}))
            .collect::<Vec<_>>());
        v["localPlayerCellId"] = json!(2);
        let d = DraftSession::parse(v).unwrap();
        assert!(d.supported);
        assert_eq!(d.ally_side, Some(Side::Red));
        assert_eq!(d.allies.len(), 5);
        assert_eq!(d.enemies.len(), 5);
        assert!(d.allies[2].local);
        assert_eq!(d.allies[2].position.as_deref(), Some("middle"));
        assert!(d.enemies.iter().all(|p| p.position.is_none() && !p.local));
    }
    #[test]
    fn masque_les_intentions_adverses_et_le_champion_avant_verrouillage() {
        let mut v = fixture();
        v["theirTeam"] = json!([{"cellId":5,"championId":157,"championPickIntent":99,"team":2}]);
        let d = DraftSession::parse(v.clone()).unwrap();
        assert_eq!(d.enemies[0].champion_id, None);
        v["actions"] = json!([[{"actorCellId":5,"championId":157,"type":"pick","completed":true}]]);
        assert_eq!(
            DraftSession::parse(v).unwrap().enemies[0].champion_id,
            Some(157)
        );
    }
    #[test]
    fn refuse_une_ressource_invalide_et_signale_les_modes_alternatifs() {
        assert!(DraftSession::parse(json!({})).is_none());
        assert!(DraftSession::parse(Value::Null).is_none());
        let mut v = fixture();
        v["benchEnabled"] = json!(true);
        assert!(!DraftSession::parse(v).unwrap().supported);
    }
    #[test]
    fn garde_les_bans_publics_et_le_timer_fini() {
        let d = DraftSession::parse(fixture()).unwrap();
        assert_eq!(d.ally_bans, vec![103, 12, 34]);
        assert_eq!(d.enemy_bans, Vec::<u32>::new());
        assert_eq!(d.timer.unwrap().remaining_ms, 36183);
        let mut v = fixture();
        v["timer"]["isInfinite"] = json!(true);
        assert!(DraftSession::parse(v).unwrap().timer.is_none());
    }
}
