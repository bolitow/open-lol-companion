//! Lecture minimale des réponses Riot et contrôle du périmètre d'une partie.
//!
//! Seuls les champs nécessaires au contrôle et aux colonnes indexées sont typés ;
//! la réponse complète est conservée en JSONB. Schémas : `LeagueEntryDTO`,
//! `MatchDto` et `TimelineDto` du catalogue officiel (league-v4, match-v5).

use serde::Deserialize;
use serde_json::Value;

/// Entrée d'un classement league-v4. `puuid` est optionnel par tolérance.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeagueEntry {
    #[serde(default)]
    pub puuid: Option<String>,
    #[serde(default)]
    pub league_points: Option<i32>,
}

/// Périmètre d'une exécution, fenêtre en millisecondes Unix `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub platform_id: String,
    pub queue_id: i32,
    pub patches: Vec<String>,
    pub window_start_ms: i64,
    pub window_end_ms: i64,
}

impl Scope {
    /// Préfixe attendu des identifiants de parties (ex. `EUW1_`).
    pub fn has_platform_prefix(&self, match_id: &str) -> bool {
        match_id
            .split_once('_')
            .is_some_and(|(prefix, _)| prefix == self.platform_id)
    }

    fn contains(&self, facts: &MatchFacts) -> Result<(), Exclusion> {
        if facts.platform_id != self.platform_id {
            return Err(Exclusion::WrongPlatform);
        }
        if self.queue_id != 0 && facts.queue_id != self.queue_id {
            return Err(Exclusion::WrongQueue);
        }
        if !self.patches.is_empty() && !self.patches.contains(&facts.patch) {
            return Err(Exclusion::WrongPatch);
        }
        if facts.game_start_ms < self.window_start_ms || facts.game_start_ms >= self.window_end_ms {
            return Err(Exclusion::OutOfWindow);
        }
        Ok(())
    }

    /// Contrôle une partie déjà en base (colonnes indexées).
    pub fn check_stored(&self, facts: &MatchFacts) -> Result<(), Exclusion> {
        self.contains(facts)
    }
}

/// Raison pour laquelle une partie valide sort du périmètre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exclusion {
    WrongPlatform,
    WrongQueue,
    WrongPatch,
    OutOfWindow,
}

impl Exclusion {
    /// Valeur enregistrée dans `collection_jobs.outcome`.
    pub fn outcome(self) -> &'static str {
        match self {
            Exclusion::WrongPlatform => "excluded:wrong_platform",
            Exclusion::WrongQueue => "excluded:wrong_queue",
            Exclusion::WrongPatch => "excluded:wrong_patch",
            Exclusion::OutOfWindow => "excluded:out_of_window",
        }
    }
}

/// Files où le rang des participants est exploité (Solo/Duo et Flex) ; les autres
/// modes n'ont pas de rang compétitif applicable, inutile de le demander (#90).
pub const RANKED_QUEUE_IDS: [i32; 2] = [420, 440];

/// Résultat du contrôle d'une réponse de détail.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCheck {
    Accepted(MatchFacts, Value),
    /// Les faits de la partie accompagnent le verdict : le cache négatif les conserve.
    Excluded(Exclusion, MatchFacts),
}

/// Colonnes indexées d'une partie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchFacts {
    pub match_id: String,
    pub platform_id: String,
    pub queue_id: i32,
    pub game_version: String,
    pub patch: String,
    /// Référence de la fenêtre : `gameStartTimestamp`, ou `gameCreation` à défaut.
    pub game_start_ms: i64,
    pub game_duration_s: i32,
    /// Partie annulée (remake) : conservée, à écarter des statistiques (#18).
    pub is_remake: bool,
    pub data_version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchDto {
    metadata: MatchMetadata,
    info: MatchInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchMetadata {
    match_id: String,
    #[serde(default)]
    data_version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchInfo {
    platform_id: String,
    queue_id: i32,
    game_creation: i64,
    #[serde(default)]
    game_start_timestamp: Option<i64>,
    #[serde(default)]
    game_end_timestamp: Option<i64>,
    game_duration: i64,
    game_version: String,
    participants: Vec<ParticipantDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParticipantDto {
    participant_id: u32,
    #[serde(default)]
    game_ended_in_early_surrender: bool,
}

/// Patch `majeur.mineur` tiré de `gameVersion` (ex. `15.19.715.1234` → `15.19`).
pub fn patch_from_version(game_version: &str) -> Option<String> {
    let mut parts = game_version.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;
    let numeric = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (numeric(major) && numeric(minor)).then(|| format!("{major}.{minor}"))
}

/// Lit et contrôle le détail d'une partie. `Err` = réponse invalide (à ne pas
/// compter comme acquise).
pub fn check_match(body: &[u8], expected_id: &str, scope: &Scope) -> Result<MatchCheck, String> {
    let raw: Value = serde_json::from_slice(body).map_err(|e| format!("JSON illisible : {e}"))?;
    let dto: MatchDto =
        serde_json::from_value(raw.clone()).map_err(|e| format!("détail incomplet : {e}"))?;
    if dto.metadata.match_id != expected_id {
        return Err("identifiant de partie différent de celui demandé".to_owned());
    }
    if !(1..=64).contains(&dto.info.participants.len()) {
        return Err(format!(
            "{} participants (attendu : 1 à 64)",
            dto.info.participants.len()
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    if dto
        .info
        .participants
        .iter()
        .any(|p| p.participant_id == 0 || p.participant_id > 64 || !ids.insert(p.participant_id))
    {
        return Err("identifiants de participants invalides".into());
    }
    let patch = patch_from_version(&dto.info.game_version)
        .ok_or_else(|| "gameVersion illisible".to_owned())?;
    // Riot : `gameDuration` est en secondes si `gameEndTimestamp` est présent,
    // en millisecondes sinon (anciennes parties).
    let duration_s = if dto.info.game_end_timestamp.is_some() {
        dto.info.game_duration
    } else {
        dto.info.game_duration / 1000
    };
    let facts = MatchFacts {
        match_id: dto.metadata.match_id,
        platform_id: dto.info.platform_id,
        queue_id: dto.info.queue_id,
        game_version: dto.info.game_version,
        patch,
        game_start_ms: dto
            .info
            .game_start_timestamp
            .unwrap_or(dto.info.game_creation),
        game_duration_s: i32::try_from(duration_s).map_err(|_| "durée hors limites".to_owned())?,
        is_remake: dto
            .info
            .participants
            .iter()
            .any(|p| p.game_ended_in_early_surrender),
        data_version: dto.metadata.data_version,
    };
    Ok(match scope.contains(&facts) {
        Ok(()) => MatchCheck::Accepted(facts, raw),
        Err(reason) => MatchCheck::Excluded(reason, facts),
    })
}

#[derive(Deserialize)]
struct TimelineDto {
    metadata: TimelineMetadata,
    info: TimelineInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineMetadata {
    match_id: String,
}

#[derive(Deserialize)]
struct TimelineInfo {
    frames: Vec<Value>,
}

/// Lit et contrôle une timeline ; renvoie la réponse complète.
pub fn check_timeline(body: &[u8], expected_id: &str) -> Result<Value, String> {
    let raw: Value = serde_json::from_slice(body).map_err(|e| format!("JSON illisible : {e}"))?;
    let dto: TimelineDto =
        serde_json::from_value(raw.clone()).map_err(|e| format!("timeline incomplète : {e}"))?;
    if dto.metadata.match_id != expected_id {
        return Err("identifiant de partie différent de celui demandé".to_owned());
    }
    if dto.info.frames.is_empty() {
        return Err("timeline sans frames".to_owned());
    }
    Ok(raw)
}

/// Lit une page de classement.
pub fn parse_league_entries(body: &[u8]) -> Result<Vec<LeagueEntry>, String> {
    let value: Value =
        serde_json::from_slice(body).map_err(|_| "classement illisible".to_owned())?;
    let entries = if value.is_array() {
        value
    } else {
        value
            .get("entries")
            .cloned()
            .ok_or("classement sans entrées")?
    };
    serde_json::from_value(entries).map_err(|_| "entrées de classement illisibles".to_owned())
}

/// Lit une liste d'identifiants de parties.
pub fn parse_match_ids(body: &[u8]) -> Result<Vec<String>, String> {
    serde_json::from_slice(body).map_err(|e| format!("liste de parties illisible : {e}"))
}

/// Classement observé lors de l'appel, jamais un rang historique du match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantRank {
    pub queue_id: i32,
    pub tier: String,
    pub division: String,
    pub league_points: i32,
    /// Champs league-v4 facultatifs : `None` si Riot ne les envoie pas ou les envoie mal formés.
    pub wins: Option<u32>,
    pub losses: Option<u32>,
    pub hot_streak: Option<bool>,
    pub veteran: Option<bool>,
    pub fresh_blood: Option<bool>,
    pub inactive: Option<bool>,
}

/// Compteur league-v4 facultatif : un entier négatif ou hors plage n'est pas retenu.
fn optional_count(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
}

fn optional_flag(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

pub fn parse_participant_ranks(body: &[u8]) -> Result<Vec<ParticipantRank>, String> {
    let values: Vec<Value> =
        serde_json::from_slice(body).map_err(|_| "classements illisibles".to_owned())?;
    let mut result = Vec::new();
    for value in values {
        let queue_id = match value.get("queueType").and_then(Value::as_str) {
            Some("RANKED_SOLO_5x5") => 420,
            Some("RANKED_FLEX_SR") => 440,
            Some(_) => continue,
            None => return Err("file de classement absente".into()),
        };
        let tier = value
            .get("tier")
            .and_then(Value::as_str)
            .ok_or("rang absent")?;
        let division = value
            .get("rank")
            .and_then(Value::as_str)
            .ok_or("division absente")?;
        let points = value
            .get("leaguePoints")
            .and_then(Value::as_i64)
            .ok_or("points absents")?;
        tier.parse::<crate::config::Tier>()
            .map_err(|_| "rang invalide")?;
        division
            .parse::<crate::config::Division>()
            .map_err(|_| "division invalide")?;
        if points < i32::MIN as i64
            || points > i32::MAX as i64
            || result
                .iter()
                .any(|r: &ParticipantRank| r.queue_id == queue_id)
        {
            return Err("classements incohérents".into());
        }
        result.push(ParticipantRank {
            queue_id,
            tier: tier.into(),
            division: division.into(),
            league_points: points as i32,
            wins: optional_count(&value, "wins"),
            losses: optional_count(&value, "losses"),
            hot_streak: optional_flag(&value, "hotStreak"),
            veteran: optional_flag(&value, "veteran"),
            fresh_blood: optional_flag(&value, "freshBlood"),
            inactive: optional_flag(&value, "inactive"),
        });
    }
    Ok(result)
}

/// Réponses synthétiques conformes aux schémas Riot, sans donnée de joueur réelle.
#[cfg(any(test, feature = "test-fixtures"))]
pub mod fixtures {
    use serde_json::{json, Value};

    /// Détail minimal d'une partie. `start_ms` alimente `gameStartTimestamp`.
    pub fn match_detail(match_id: &str, platform: &str, queue: i32, start_ms: i64) -> Value {
        let participants: Vec<Value> = (0..10)
            .map(|i| {
                let position = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"][i % 5];
                json!({
                    "participantId": i + 1,
                    "puuid": format!("fake-puuid-{i}"),
                    "riotIdGameName": format!("Joueur{i}"),
                    "teamId": if i < 5 { 100 } else { 200 },
                    "teamPosition": position,
                    "championId": 1 + i,
                    "win": i < 5,
                    "gameEndedInEarlySurrender": false
                })
            })
            .collect();
        json!({
            "metadata": {
                "dataVersion": "2",
                "matchId": match_id,
                "participants": (0..10).map(|i| format!("fake-puuid-{i}")).collect::<Vec<_>>()
            },
            "info": {
                "endOfGameResult": "GameComplete",
                "gameCreation": start_ms - 60_000,
                "gameStartTimestamp": start_ms,
                "gameEndTimestamp": start_ms + 1_800_000,
                "gameDuration": 1800,
                "gameVersion": "15.19.715.1234",
                "platformId": platform,
                "queueId": queue,
                "participants": participants
            }
        })
    }

    /// Timeline minimale d'une partie.
    pub fn timeline(match_id: &str) -> Value {
        json!({
            "metadata": { "dataVersion": "2", "matchId": match_id, "participants": [] },
            "info": {
                "frameInterval": 60000,
                "frames": [
                    { "timestamp": 0, "events": [], "participantFrames": {} },
                    { "timestamp": 60000, "events": [{ "type": "ITEM_PURCHASED", "timestamp": 1500 }], "participantFrames": {} }
                ]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    fn scope() -> Scope {
        Scope {
            platform_id: "EUW1".into(),
            queue_id: 420,
            patches: vec![],
            window_start_ms: 1_000_000,
            window_end_ms: 2_000_000,
        }
    }

    fn body(v: &Value) -> Vec<u8> {
        serde_json::to_vec(v).unwrap()
    }

    #[test]
    fn accepte_une_reponse_solo_et_refuse_plus_de_soixante_quatre_participants() {
        // Fixture synthétique : le catalogue officiel décrit 1810 comme Swarm solo.
        let mut value = match_detail("EUW1_1", "EUW1", 1810, 1_500_000);
        let mut all = scope();
        all.queue_id = 0;
        value["info"]["participants"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        assert!(matches!(
            check_match(&body(&value), "EUW1_1", &all),
            Ok(MatchCheck::Accepted(_, _))
        ));
        let first = value["info"]["participants"][0].clone();
        for n in 2..=65 {
            let mut participant = first.clone();
            participant["participantId"] = serde_json::json!(n);
            value["info"]["participants"]
                .as_array_mut()
                .unwrap()
                .push(participant);
        }
        assert!(check_match(&body(&value), "EUW1_1", &all).is_err());
    }

    #[test]
    fn conserve_un_solde_de_lp_negatif() {
        let ranks = parse_participant_ranks(
            br#"[{"queueType":"RANKED_FLEX_SR","tier":"GOLD","rank":"II","leaguePoints":-5}]"#,
        )
        .unwrap();
        assert_eq!(ranks[0].league_points, -5);
    }

    #[test]
    fn refuse_identifiants_de_participants_absents_ou_dupliques() {
        let mut value = match_detail("EUW1_1", "EUW1", 420, 1_500_000);
        value["info"]["participants"][0]["participantId"] = serde_json::json!(2);
        assert!(check_match(&body(&value), "EUW1_1", &scope()).is_err());
    }

    #[test]
    fn lit_victoires_defaites_et_drapeaux_league_v4() {
        let ranks = parse_participant_ranks(
            br#"[{"queueType":"RANKED_SOLO_5x5","tier":"GOLD","rank":"II","leaguePoints":42,
                 "wins":12,"losses":9,"hotStreak":true,"veteran":false,"freshBlood":true,"inactive":false}]"#,
        )
        .unwrap();
        assert_eq!(ranks[0].wins, Some(12));
        assert_eq!(ranks[0].losses, Some(9));
        assert_eq!(ranks[0].hot_streak, Some(true));
        assert_eq!(ranks[0].veteran, Some(false));
        assert_eq!(ranks[0].fresh_blood, Some(true));
        assert_eq!(ranks[0].inactive, Some(false));
    }

    #[test]
    fn laisse_vides_les_champs_league_v4_absents_ou_invalides() {
        let ranks = parse_participant_ranks(
            br#"[{"queueType":"RANKED_FLEX_SR","tier":"GOLD","rank":"II","leaguePoints":1,
                 "wins":-3,"losses":"beaucoup","hotStreak":"oui"}]"#,
        )
        .unwrap();
        assert_eq!(ranks[0].wins, None);
        assert_eq!(ranks[0].losses, None);
        assert_eq!(ranks[0].hot_streak, None);
        assert_eq!(ranks[0].veteran, None);
        assert_eq!(ranks[0].fresh_blood, None);
        assert_eq!(ranks[0].inactive, None);
    }

    #[test]
    fn lit_les_deux_classements_sans_inventer_un_historique() {
        let entries = parse_participant_ranks(
            br#"[{"queueType":"RANKED_SOLO_5x5","tier":"GOLD","rank":"II","leaguePoints":42}]"#,
        )
        .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].queue_id, 420);
        assert_eq!(entries[0].tier, "GOLD");
        assert!(parse_participant_ranks(b"[]").unwrap().is_empty());
        assert!(parse_participant_ranks(br#"[{"queueType":"RANKED_SOLO_5x5"}]"#).is_err());
    }

    #[test]
    fn accepte_une_partie_a_seize_participants() {
        let mut value = match_detail("EUW1_1", "EUW1", 420, 1_500_000);
        let participants = value["info"]["participants"].as_array_mut().unwrap();
        for n in 10..16 {
            let mut participant = participants[0].clone();
            participant["puuid"] = serde_json::json!(format!("fake-puuid-{n}"));
            participant["participantId"] = serde_json::json!(n + 1);
            participants.push(participant);
        }
        assert!(matches!(
            check_match(&body(&value), "EUW1_1", &scope()),
            Ok(MatchCheck::Accepted(_, _))
        ));
    }

    #[test]
    fn accepte_une_partie_du_perimetre() {
        let b = body(&match_detail("EUW1_1", "EUW1", 420, 1_500_000));
        let MatchCheck::Accepted(facts, raw) = check_match(&b, "EUW1_1", &scope()).unwrap() else {
            panic!("partie refusée")
        };
        assert_eq!(facts.patch, "15.19");
        assert_eq!(facts.game_duration_s, 1800);
        assert_eq!(facts.game_start_ms, 1_500_000);
        assert!(!facts.is_remake);
        assert_eq!(facts.data_version.as_deref(), Some("2"));
        // La réponse complète est conservée, champs non typés compris.
        assert_eq!(raw["info"]["endOfGameResult"], "GameComplete");
    }

    #[test]
    fn exclut_plateforme_file_et_fenetre() {
        let s = scope();
        let check = |v: Value| check_match(&body(&v), "EUW1_1", &s).unwrap();
        let reason = |v: Value| match check(v) {
            MatchCheck::Excluded(reason, _) => Some(reason),
            MatchCheck::Accepted(..) => None,
        };
        assert_eq!(
            reason(match_detail("EUW1_1", "EUN1", 420, 1_500_000)),
            Some(Exclusion::WrongPlatform)
        );
        assert_eq!(
            reason(match_detail("EUW1_1", "EUW1", 440, 1_500_000)),
            Some(Exclusion::WrongQueue)
        );
        assert_eq!(
            reason(match_detail("EUW1_1", "EUW1", 420, 999_999)),
            Some(Exclusion::OutOfWindow)
        );
        // Borne de fin exclue.
        assert_eq!(
            reason(match_detail("EUW1_1", "EUW1", 420, 2_000_000)),
            Some(Exclusion::OutOfWindow)
        );
    }

    #[test]
    fn une_partie_exclue_garde_ses_faits_pour_le_cache_negatif() {
        let s = scope();
        let check = check_match(
            &body(&match_detail("EUW1_1", "EUW1", 440, 1_500_000)),
            "EUW1_1",
            &s,
        )
        .unwrap();
        let MatchCheck::Excluded(reason, facts) = check else {
            panic!("la file 440 doit être exclue d'un périmètre 420");
        };
        assert_eq!(reason, Exclusion::WrongQueue);
        assert_eq!(facts.queue_id, 440);
        assert_eq!(facts.game_start_ms, 1_500_000);
        // Les faits suffisent à rejuger la partie dans un autre périmètre.
        let all_queues = Scope { queue_id: 0, ..s };
        assert_eq!(all_queues.check_stored(&facts), Ok(()));
    }

    #[test]
    fn refuse_une_reponse_incoherente_ou_invalide() {
        let s = scope();
        let other = body(&match_detail("EUW1_2", "EUW1", 420, 1_500_000));
        assert!(check_match(&other, "EUW1_1", &s).is_err());
        assert!(check_match(b"{pas du json", "EUW1_1", &s).is_err());
        assert!(check_match(b"{}", "EUW1_1", &s).is_err());
        let mut nine = match_detail("EUW1_1", "EUW1", 420, 1_500_000);
        nine["info"]["participants"].as_array_mut().unwrap().clear();
        assert!(check_match(&body(&nine), "EUW1_1", &s).is_err());
    }

    #[test]
    fn repere_les_remakes_et_les_durees_en_millisecondes() {
        let mut v = match_detail("EUW1_1", "EUW1", 420, 1_500_000);
        v["info"]["participants"][3]["gameEndedInEarlySurrender"] = Value::Bool(true);
        v["info"]
            .as_object_mut()
            .unwrap()
            .remove("gameEndTimestamp");
        v["info"]["gameDuration"] = 200_000.into();
        let MatchCheck::Accepted(facts, _) = check_match(&body(&v), "EUW1_1", &scope()).unwrap()
        else {
            panic!("partie refusée")
        };
        assert!(facts.is_remake);
        assert_eq!(facts.game_duration_s, 200);
    }

    #[test]
    fn calcule_le_patch() {
        assert_eq!(
            patch_from_version("15.19.715.1234").as_deref(),
            Some("15.19")
        );
        assert_eq!(patch_from_version("15"), None);
        assert_eq!(patch_from_version("a.b.c"), None);
    }

    #[test]
    fn verifie_le_prefixe_de_plateforme() {
        let s = scope();
        assert!(s.has_platform_prefix("EUW1_7412345678"));
        assert!(!s.has_platform_prefix("EUN1_7412345678"));
        assert!(!s.has_platform_prefix("EUW17412345678"));
    }

    #[test]
    fn controle_la_timeline() {
        assert!(check_timeline(&body(&timeline("EUW1_1")), "EUW1_1").is_ok());
        assert!(check_timeline(&body(&timeline("EUW1_2")), "EUW1_1").is_err());
        let mut empty = timeline("EUW1_1");
        empty["info"]["frames"] = Value::Array(vec![]);
        assert!(check_timeline(&body(&empty), "EUW1_1").is_err());
    }

    #[test]
    fn lit_classements_et_listes_de_parties() {
        let entries = parse_league_entries(
            br#"[{"puuid":"p1","leaguePoints":42,"tier":"GOLD","rank":"I"},{"leaguePoints":3}]"#,
        )
        .unwrap();
        assert_eq!(entries[0].puuid.as_deref(), Some("p1"));
        assert_eq!(entries[0].league_points, Some(42));
        assert_eq!(entries[1].puuid, None);
        assert_eq!(
            parse_match_ids(br#"["EUW1_1","EUW1_2"]"#).unwrap(),
            vec!["EUW1_1", "EUW1_2"]
        );
        assert!(parse_match_ids(b"{}").is_err());
    }
}
