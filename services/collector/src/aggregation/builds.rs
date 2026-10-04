//! Projection des choix observés, sans identifiant de joueur ni recommandation automatique.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub(super) struct BuildObservation {
    pub variants: BTreeMap<String, Vec<u32>>,
    pub skill_steps: Vec<SkillStep>,
    pub item_events: Vec<ItemEvent>,
    pub unidentified_item_undos: u64,
    /// Achats nets horodatés, présents seulement si `purchase_order` est publiable.
    /// Projection interne des étapes (#81) : jamais sérialisée.
    #[serde(skip)]
    pub net_purchases: Option<Vec<Purchase>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Purchase {
    pub item_id: u32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct SkillStep {
    /// Ordre des points investis, pas niveau du champion : un joueur peut les conserver.
    pub level: u32,
    pub slot: u32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ItemEvent {
    pub kind: String,
    pub item_id: u32,
    pub timestamp_ms: u64,
}

pub(super) fn extract_detail(participant: &Value) -> BuildObservation {
    let mut result = BuildObservation::default();
    // Chaque catégorie est atomique : six cases présentes dont certaines vides sont
    // un inventaire connu ; une case absente n'est pas assimilée à une case vide.
    let items: Option<Vec<_>> = (0..6)
        .map(|slot| unsigned(&participant[format!("item{slot}")]))
        .collect();
    if let Some(mut items) = items {
        items.retain(|item| *item > 0);
        items.sort_unstable();
        items.dedup();
        result.variants.insert("final_items".into(), items);
    }
    if let Some(item) = unsigned(&participant["item6"]) {
        result.variants.insert(
            "trinket".into(),
            if item == 0 { Vec::new() } else { vec![item] },
        );
    }
    if let (Some(first), Some(second)) = (
        positive(&participant["summoner1Id"]),
        positive(&participant["summoner2Id"]),
    ) {
        let mut spells = vec![first, second];
        spells.sort_unstable();
        result.variants.insert("summoner_spells".into(), spells);
    }
    if let Some(runes) = extract_runes(&participant["perks"]) {
        result.variants.extend(derive_rune_choices(&runes));
        result.variants.insert("runes".into(), runes);
    }
    result
}

/// Choix de runes (#86) dérivés de la page exacte de 11 identifiants, sans nouvelle
/// collecte : la page exacte fragmente la population dès qu'un fragment diffère, ces
/// catégories donnent à chaque choix son propre effectif. Les runes d'emplacement
/// portent leur clé de voûte pour rester conditionnelles ; la paire secondaire est triée,
/// l'ordre transmis par Riot n'étant pas un choix du joueur.
fn derive_rune_choices(page: &[u32]) -> BTreeMap<String, Vec<u32>> {
    // Ordre de `extract_runes` : arbre principal, 4 runes, arbre secondaire, 2 runes, 3 fragments.
    let [primary, keystone, slot_1, slot_2, slot_3, secondary, first, second, offense, flex, defense] =
        *page
    else {
        return BTreeMap::new();
    };
    let mut pair = [first, second];
    pair.sort_unstable();
    BTreeMap::from([
        ("rune_keystone".into(), vec![keystone]),
        ("rune_primary_style".into(), vec![primary]),
        ("rune_secondary_style".into(), vec![secondary]),
        (
            "rune_secondary_pair".into(),
            vec![secondary, pair[0], pair[1]],
        ),
        ("rune_slot_1".into(), vec![keystone, slot_1]),
        ("rune_slot_2".into(), vec![keystone, slot_2]),
        ("rune_slot_3".into(), vec![keystone, slot_3]),
        ("rune_shard_offense".into(), vec![offense]),
        ("rune_shard_flex".into(), vec![flex]),
        ("rune_shard_defense".into(), vec![defense]),
    ])
}

/// Schéma match-v5, champs spécialisés confirmés sur la recette du 1er octobre 2026.
/// Les types inconnus n'influencent aucune catégorie ; les événements reconnus
/// incomplets font échouer la projection, pour ne pas publier une séquence tronquée.
pub(super) fn extract_timeline(
    timeline: &Value,
    match_id: &str,
    participant_id: u32,
) -> Result<BuildObservation, &'static str> {
    const INVALID: &str = "invalid_timeline";
    if match_id.is_empty()
        || timeline["metadata"]["matchId"].as_str() != Some(match_id)
        || participant_id == 0
    {
        return Err(INVALID);
    }
    let participants = timeline["info"]["participants"].as_array().ok_or(INVALID)?;
    let mut ids = BTreeSet::new();
    for participant in participants {
        if !ids.insert(positive(&participant["participantId"]).ok_or(INVALID)?) {
            return Err(INVALID);
        }
    }
    if !ids.contains(&participant_id) {
        return Err(INVALID);
    }
    let frames = timeline["info"]["frames"].as_array().ok_or(INVALID)?;
    let mut events = Vec::new();
    for frame in frames {
        for event in frame["events"].as_array().ok_or(INVALID)? {
            let kind = event["type"].as_str().unwrap_or_default();
            if !matches!(
                kind,
                "SKILL_LEVEL_UP" | "ITEM_PURCHASED" | "ITEM_SOLD" | "ITEM_DESTROYED" | "ITEM_UNDO"
            ) {
                continue;
            }
            // Zéro désigne un événement système dans les réponses Riot observées,
            // sans joueur auquel l'attribuer. Une valeur absente ou malformée reste invalide.
            let event_participant = unsigned(&event["participantId"]).ok_or(INVALID)?;
            if event_participant != participant_id {
                continue;
            }
            let timestamp = event["timestamp"].as_u64().ok_or(INVALID)?;
            events.push((timestamp, kind, event));
        }
    }
    // Tri stable : à temps égal, l'ordre transmis par Riot départage les actions.
    events.sort_by_key(|(timestamp, _, _)| *timestamp);
    let mut result = BuildObservation::default();
    let mut purchases = Vec::new();
    let mut skills = Vec::new();
    let mut special_skills = Vec::new();
    let mut has_purchase = false;
    for (timestamp_ms, kind, event) in events {
        match kind {
            "SKILL_LEVEL_UP" => {
                let slot = positive(&event["skillSlot"])
                    .filter(|slot| *slot <= 4)
                    .ok_or(INVALID)?;
                // Les 18 niveaux usuels ne couvrent pas tous les modes/champions.
                // La borne défensive de 64 refuse, sans tronquer, une suite incohérente.
                if skills.len() + special_skills.len() >= 64 {
                    return Err(INVALID);
                }
                match event["levelUpType"].as_str() {
                    Some("NORMAL") => {
                        skills.push(slot);
                        result.skill_steps.push(SkillStep {
                            level: skills.len() as u32,
                            slot,
                            timestamp_ms,
                        });
                    }
                    Some("EVOLVE") => special_skills.push(slot),
                    _ => return Err(INVALID),
                }
            }
            "ITEM_UNDO" => {
                let before = unsigned(&event["beforeId"]).ok_or(INVALID)?;
                let after = unsigned(&event["afterId"]).ok_or(INVALID)?;
                if before == 0 && after == 0 {
                    // Réponses réelles : remboursement d'or sans objet identifiable.
                    // Préserver les sorts et événements, mais l'ordre net des achats
                    // ne peut plus être garanti sans deviner l'objet remboursé.
                    if event["goldGain"].as_i64().filter(|g| *g != 0).is_none() {
                        return Err(INVALID);
                    }
                    result.unidentified_item_undos += 1;
                    continue;
                }
                if before > 0 {
                    let index = purchases
                        .iter()
                        .rposition(|p: &Purchase| p.item_id == before)
                        .ok_or(INVALID)?;
                    purchases.remove(index);
                    result.item_events.push(ItemEvent {
                        kind: "ITEM_UNDO_REMOVE".into(),
                        item_id: before,
                        timestamp_ms,
                    });
                }
                // Annuler une vente restaure l'objet mais ne constitue pas un achat.
                if after > 0 {
                    result.item_events.push(ItemEvent {
                        kind: "ITEM_UNDO_RESTORE".into(),
                        item_id: after,
                        timestamp_ms,
                    });
                }
            }
            _ => {
                let item_id = positive(&event["itemId"]).ok_or(INVALID)?;
                if kind == "ITEM_PURCHASED" {
                    purchases.push(Purchase {
                        item_id,
                        timestamp_ms,
                    });
                    has_purchase = true;
                }
                // Une vente ou consommation ne retire pas un achat de son historique.
                result.item_events.push(ItemEvent {
                    kind: kind.into(),
                    item_id,
                    timestamp_ms,
                });
            }
        }
    }
    if has_purchase && result.unidentified_item_undos == 0 {
        result.variants.insert(
            "purchase_order".into(),
            purchases.iter().map(|p| p.item_id).collect(),
        );
        result.net_purchases = Some(purchases);
    }
    if !skills.is_empty() {
        result.variants.insert("skill_order".into(), skills);
    }
    if !special_skills.is_empty() {
        result
            .variants
            .insert("special_skill_order".into(), special_skills);
    }
    Ok(result)
}

fn unsigned(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

fn positive(value: &Value) -> Option<u32> {
    unsigned(value).filter(|value| *value > 0)
}

fn extract_runes(perks: &Value) -> Option<Vec<u32>> {
    let styles = perks["styles"].as_array()?;
    if styles.len() != 2 {
        return None;
    }
    let mut result = Vec::new();
    for (description, expected) in [("primaryStyle", 4), ("subStyle", 2)] {
        let style = styles
            .iter()
            .find(|style| style["description"] == description)?;
        result.push(positive(&style["style"])?);
        let selections = style["selections"].as_array()?;
        if selections.len() != expected {
            return None;
        }
        for selection in selections {
            result.push(positive(&selection["perk"])?);
        }
    }
    for stat in ["offense", "flex", "defense"] {
        result.push(positive(&perks["statPerks"][stat])?);
    }
    Some(result)
}

#[cfg(test)]
#[path = "builds_tests.rs"]
mod tests;
