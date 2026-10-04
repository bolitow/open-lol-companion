use super::*;
use serde_json::json;

#[test]
fn les_tranches_ont_leur_borne_basse_incluse_et_ignorent_une_duree_inconnue() {
    use SplitBucket::*;
    for (seconds, expected) in [
        (i32::MIN, None),
        (-5, None),
        (0, None),
        (1, Some(Under20)),
        (1199, Some(Under20)),
        (1200, Some(From20To25)),
        (1499, Some(From20To25)),
        (1500, Some(From25To30)),
        (1799, Some(From25To30)),
        (1800, Some(From30To35)),
        (2099, Some(From30To35)),
        (2100, Some(From35To40)),
        (2399, Some(From35To40)),
        (2400, Some(From40)),
        (i32::MAX, Some(From40)),
    ] {
        assert_eq!(SplitBucket::from_duration_s(seconds), expected, "{seconds}");
    }
}

#[test]
fn les_etiquettes_publiees_et_les_axes_sont_stables() {
    let labels: Vec<_> = [
        SplitBucket::Under20,
        SplitBucket::From20To25,
        SplitBucket::From25To30,
        SplitBucket::From30To35,
        SplitBucket::From35To40,
        SplitBucket::From40,
        SplitBucket::Blue,
        SplitBucket::Red,
    ]
    .iter()
    .map(|b| (serde_json::to_value(b).unwrap(), b.dimension()))
    .collect();
    let expected = [
        "lt_20", "20_25", "25_30", "30_35", "35_40", "gte_40", "blue", "red",
    ];
    for ((label, dimension), expected) in labels.iter().zip(expected) {
        assert_eq!(label, expected);
        let side = ["blue", "red"].contains(&expected);
        assert_eq!(
            *dimension,
            if side {
                SplitDimension::Side
            } else {
                SplitDimension::Duration
            }
        );
    }
    assert_eq!(SplitBucket::from_team(100), Some(SplitBucket::Blue));
    assert_eq!(SplitBucket::from_team(200), Some(SplitBucket::Red));
    assert_eq!(SplitBucket::from_team(1), None);
}

#[test]
fn la_premiere_equipe_exige_deux_equipes_et_un_seul_premier() {
    let detail = |teams: serde_json::Value| json!({"info": {"teams": teams}});
    let team = |id: u32, first: serde_json::Value| json!({"teamId": id, "objectives": {"dragon": {"first": first}}});
    assert_eq!(
        first_team(
            &detail(json!([team(100, json!(false)), team(200, json!(true))])),
            "dragon"
        ),
        Some(200)
    );
    // L'ordre du tableau ne compte pas.
    assert_eq!(
        first_team(
            &detail(json!([team(200, json!(false)), team(100, json!(true))])),
            "dragon"
        ),
        Some(100)
    );
    for invalid in [
        json!([team(100, json!(true)), team(200, json!(true))]),
        json!([team(100, json!(false)), team(200, json!(false))]),
        json!([team(100, json!(true)), team(100, json!(false))]),
        json!([team(100, json!(true)), team(300, json!(false))]),
        json!([team(100, json!(true))]),
        json!([team(100, json!("oui")), team(200, json!(false))]),
        json!("pas un tableau"),
    ] {
        assert_eq!(first_team(&detail(invalid), "dragon"), None);
    }
    assert_eq!(first_team(&json!({}), "dragon"), None);
    // Un objectif absent des deux équipes n'est pas deviné.
    assert_eq!(
        first_team(
            &detail(json!([team(100, json!(true)), team(200, json!(false))])),
            "tower"
        ),
        None
    );
}

#[test]
fn les_taux_des_premiers_objectifs_respectent_le_seuil() {
    let mut stats = FirstObjectiveStats::default();
    stats.record(true, true);
    stats.record(false, false);
    stats.record(true, false);
    stats.finish(2);
    assert_eq!((stats.matches, stats.wins), (3, 1));
    assert_eq!((stats.blue_matches, stats.blue_wins), (2, 1));
    assert!((stats.win_rate.unwrap() - 100.0 / 3.0).abs() < 1e-10);
    assert_eq!(stats.blue_win_rate, Some(50.0));
    stats.finish(3);
    assert_eq!(stats.blue_win_rate, None);
}
