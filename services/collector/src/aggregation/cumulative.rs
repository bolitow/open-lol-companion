//! Paliers cumulés (#83) : « Émeraude et plus », « Diamant et plus »… Les paliers observés
//! partitionnent les participations classées ; un palier cumulé est la réunion du palier
//! de son seuil et de tous les paliers supérieurs. Ce n'est qu'un regroupement de paliers
//! observés, jamais un MMR estimé, et jamais additionné à `ALL` ni à un autre palier.

use std::collections::BTreeMap;

use super::match_tier::TIERS;

/// Paliers cumulés publiés, du plus large au plus étroit. Pas de `GRANDMASTER_PLUS` ni de
/// `CHALLENGER_PLUS` : leur effectif est trop faible pour un filtre exploitable, et
/// `MASTER_PLUS` couvre déjà les trois paliers apex.
pub const CUMULATIVE_RANKS: [&str; 8] = [
    "IRON_PLUS",
    "BRONZE_PLUS",
    "SILVER_PLUS",
    "GOLD_PLUS",
    "PLATINUM_PLUS",
    "EMERALD_PLUS",
    "DIAMOND_PLUS",
    "MASTER_PLUS",
];

/// Paliers cumulés qui contiennent `tier` ; vide pour `ALL`, `UNKNOWN`, `UNRANKED`,
/// `UNRANKED_MODE` ou toute valeur qui n'est pas un palier observé.
pub fn containing(tier: &str) -> &'static [&'static str] {
    match TIERS.iter().position(|t| *t == tier) {
        Some(index) => &CUMULATIVE_RANKS[..=index.min(CUMULATIVE_RANKS.len() - 1)],
        None => &[],
    }
}

/// Ajoute, pour chaque entrée dont le rang est un palier observé, une entrée par palier
/// cumulé qui le contient. Les clés cumulées sont distinctes des clés observées : rien n'est
/// écrasé. À n'appliquer que sur des effectifs additifs (parties, victoires, populations,
/// bans, événements) ; les parties distinctes se comptent à l'ajout, pas ici.
pub(super) fn extend<K: Ord + Clone, V: Clone>(
    map: &mut BTreeMap<K, V>,
    rank_mut: fn(&mut K) -> &mut String,
    merge: fn(&mut V, &V),
) {
    let mut cumulated = BTreeMap::<K, V>::new();
    for (key, value) in map.iter() {
        let mut target = key.clone();
        let tier = rank_mut(&mut target).clone();
        for rank in containing(&tier) {
            *rank_mut(&mut target) = (*rank).to_owned();
            match cumulated.get_mut(&target) {
                Some(total) => merge(total, value),
                None => {
                    cumulated.insert(target.clone(), value.clone());
                }
            }
        }
    }
    map.append(&mut cumulated);
}
