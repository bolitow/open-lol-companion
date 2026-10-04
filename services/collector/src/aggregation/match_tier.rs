//! Palier de partie (#109) : médiane des paliers observés des joueurs, sert de clé de rang
//! au ban rate. Un ban n'a pas de rang d'auteur dans match-v5 ; le palier de la partie est le
//! palier observé de ses joueurs, jamais un MMR estimé.

/// Palier observé minimal de joueurs connus pour publier un palier de partie (6 sur 10).
pub(super) const MIN_KNOWN_PLAYERS: usize = 6;
/// Valeur publiée dans `ban_rank_basis` : le rang des bans est la médiane des paliers de la partie.
pub(super) const BAN_RANK_BASIS: &str = "match_median";

/// Paliers classés, du plus bas au plus haut.
const TIERS: [&str; 10] = [
    "IRON",
    "BRONZE",
    "SILVER",
    "GOLD",
    "PLATINUM",
    "EMERALD",
    "DIAMOND",
    "MASTER",
    "GRANDMASTER",
    "CHALLENGER",
];

/// Médiane des paliers connus ; `None` sous `MIN_KNOWN_PLAYERS` joueurs connus. Les rangs
/// `UNRANKED`, `UNKNOWN` et `UNRANKED_MODE` ne comptent pas. À effectif pair, la médiane est le
/// plus bas des deux paliers centraux : jamais un palier intermédiaire qu'aucun joueur n'a.
pub(super) fn median_tier<'a>(ranks: impl IntoIterator<Item = &'a str>) -> Option<&'static str> {
    let mut known: Vec<usize> = ranks
        .into_iter()
        .filter_map(|rank| TIERS.iter().position(|tier| *tier == rank))
        .collect();
    if known.len() < MIN_KNOWN_PLAYERS {
        return None;
    }
    known.sort_unstable();
    Some(TIERS[known[(known.len() - 1) / 2]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mediane_d_un_effectif_impair() {
        let ranks = [
            "GOLD", "GOLD", "PLATINUM", "GOLD", "DIAMOND", "PLATINUM", "GOLD",
        ];
        assert_eq!(median_tier(ranks), Some("GOLD"));
    }

    #[test]
    fn mediane_d_un_effectif_pair_prend_le_palier_central_le_plus_bas() {
        let ranks = ["PLATINUM", "SILVER", "DIAMOND", "GOLD", "PLATINUM", "GOLD"];
        assert_eq!(median_tier(ranks), Some("GOLD"));
    }

    #[test]
    fn ne_depend_pas_de_l_ordre_des_joueurs() {
        let mut ranks = ["IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD"];
        let expected = median_tier(ranks);
        ranks.reverse();
        assert_eq!(median_tier(ranks), expected);
        assert_eq!(expected, Some("SILVER"));
    }

    #[test]
    fn exige_six_joueurs_connus_sur_dix() {
        let five = ["GOLD", "GOLD", "GOLD", "GOLD", "GOLD"];
        assert_eq!(median_tier(five), None);
        // Les rangs inconnus ou non classés ne comptent pas dans la couverture.
        let padded = [
            "GOLD", "GOLD", "GOLD", "GOLD", "GOLD", "UNKNOWN", "UNRANKED", "UNKNOWN", "UNKNOWN",
            "UNKNOWN",
        ];
        assert_eq!(median_tier(padded), None);
        let enough = [
            "GOLD", "GOLD", "GOLD", "GOLD", "GOLD", "GOLD", "UNKNOWN", "UNKNOWN", "UNKNOWN",
            "UNRANKED",
        ];
        assert_eq!(median_tier(enough), Some("GOLD"));
    }

    #[test]
    fn aucun_palier_sans_joueur_classe() {
        assert_eq!(median_tier([]), None);
        assert_eq!(median_tier(["UNRANKED_MODE"; 10]), None);
    }
}
