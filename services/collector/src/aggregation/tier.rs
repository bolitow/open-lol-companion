//! Tier descriptif d'un champion dans son compartiment (#85) : winrate lissé vers la moyenne
//! du compartiment, présence (pick + ban) et seuils absolus. Aucune répartition forcée.

/// Force du lissage, en parties fictives au winrate moyen du compartiment.
pub(super) const PRIOR_GAMES: f64 = 200.0;
/// Points de score ajoutés par point de présence (pick rate + ban rate, en %).
pub(super) const PRESENCE_WEIGHT: f64 = 0.02;
/// Pick rate minimal (%) pour recevoir un tier.
pub(super) const MIN_TIER_PICK_RATE: f64 = 0.5;
/// Champions éligibles minimaux dans un compartiment pour y attribuer des tiers.
pub(super) const MIN_TIER_CHAMPIONS: u32 = 20;
/// Seuils absolus (score minimal, lettre), du plus haut au plus bas ; D en dessous.
const THRESHOLDS: [(f64, &str); 4] = [(2.5, "S"), (1.0, "A"), (-1.0, "B"), (-2.5, "C")];

/// Score en points de pourcentage : écart du winrate lissé au winrate moyen du compartiment
/// (`baseline`, en %), plus `PRESENCE_WEIGHT` × présence (pick rate + ban rate, en %).
/// Le lissage ajoute `PRIOR_GAMES` parties fictives au winrate `baseline` : un petit
/// échantillon reste proche de la moyenne, un grand garde son écart observé.
pub(super) fn tier_score(wins: u64, games: u64, baseline: f64, presence: f64) -> f64 {
    let shrunk = (100.0 * wins as f64 + PRIOR_GAMES * baseline) / (games as f64 + PRIOR_GAMES);
    shrunk - baseline + PRESENCE_WEIGHT * presence
}

/// Lettre d'un score selon les seuils absolus ; une borne exacte reçoit la lettre supérieure.
pub(super) fn tier_letter(score: f64) -> &'static str {
    THRESHOLDS
        .iter()
        .find(|(minimum, _)| score >= *minimum)
        .map_or("D", |(_, letter)| letter)
}

/// Méthode publiée dans `tier_method`, construite depuis les constantes appliquées.
pub(super) fn tier_method() -> String {
    let thresholds: Vec<String> = THRESHOLDS
        .iter()
        .map(|(minimum, letter)| format!("{letter}>={minimum}"))
        .collect();
    format!(
        "bayes_shrunk_win_rate: score = 100*(wins + {PRIOR_GAMES}*mu/100)/(games + {PRIOR_GAMES}) - mu \
         + {PRESENCE_WEIGHT}*(pick_rate + ban_rate), mu = bucket wins/games %; ban_rate of the same \
         scope and match rank for ALL, UNRANKED_MODE and ranked tiers, else 0; {} else D (absolute, \
         no forced distribution); tier requires pick_rate>={MIN_TIER_PICK_RATE} and at least \
         {MIN_TIER_CHAMPIONS} such champions in the bucket; position by score, then win rate, \
         games, champion id; Arena: position by ascending average placement, then games, \
         champion id, no tier; win_rate_lower_bound (Wilson95) published, not used",
        thresholds.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter(wins: u64, games: u64) -> &'static str {
        tier_letter(tier_score(wins, games, 50.0, 0.0))
    }

    #[test]
    fn un_fort_winrate_peu_joue_passe_devant_un_winrate_moyen_tres_joue() {
        // Contre-exemples du ticket : la borne Wilson classait l'inverse.
        assert!(tier_score(81, 150, 50.0, 0.0) > tier_score(998, 2000, 50.0, 0.0));
        assert!(tier_score(110, 200, 50.0, 0.0) > tier_score(520, 1000, 50.0, 0.0));
    }

    #[test]
    fn le_lissage_rapproche_les_petits_echantillons_de_la_moyenne() {
        // 1 victoire sur 1 : (1 + 100) / 201 → +0,25 point seulement.
        assert!((tier_score(1, 1, 50.0, 0.0) - 0.248_756).abs() < 1e-5);
        assert_eq!(tier_score(5, 10, 50.0, 0.0), 0.0);
        assert!(tier_score(0, 1, 50.0, 0.0) < 0.0);
    }

    #[test]
    fn le_score_est_relatif_au_winrate_moyen_du_compartiment() {
        assert_eq!(tier_score(450, 1000, 45.0, 0.0), 0.0);
        assert!(tier_score(500, 1000, 45.0, 0.0) > 2.5);
    }

    #[test]
    fn des_champions_entre_49_et_51_pourcent_sont_tous_b() {
        for games in [100, 1_000, 10_000, 1_000_000] {
            assert_eq!(letter(games * 49 / 100, games), "B");
            assert_eq!(letter(games * 51 / 100, games), "B");
        }
    }

    #[test]
    fn aucune_victoire_donne_d_et_que_des_victoires_donnent_s() {
        assert_eq!(letter(0, 100), "D");
        assert_eq!(letter(100, 100), "S");
        let score = tier_score(0, 1_000, 50.0, 0.0);
        assert!(score.is_finite() && score > -50.0);
    }

    #[test]
    fn la_presence_pick_et_ban_releve_le_score() {
        let base = tier_score(500, 1000, 50.0, 0.0);
        assert_eq!(base, 0.0);
        assert!((tier_score(500, 1000, 50.0, 60.0) - 1.2).abs() < 1e-9);
        assert_eq!(tier_letter(tier_score(500, 1000, 50.0, 60.0)), "A");
    }

    #[test]
    fn les_bornes_exactes_recoivent_la_lettre_superieure() {
        assert_eq!(tier_letter(2.5), "S");
        assert_eq!(tier_letter(2.499_999), "A");
        assert_eq!(tier_letter(1.0), "A");
        assert_eq!(tier_letter(0.999_999), "B");
        assert_eq!(tier_letter(-1.0), "B");
        assert_eq!(tier_letter(-1.000_001), "C");
        assert_eq!(tier_letter(-2.5), "C");
        assert_eq!(tier_letter(-2.500_001), "D");
    }
}
