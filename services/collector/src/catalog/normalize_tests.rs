use super::*;
use serde_json::{json, Value};

fn source(key: &str, data: Value) -> CatalogSource {
    let locale = key
        .split('/')
        .next()
        .filter(|s| ["fr_FR", "en_US"].contains(s));
    CatalogSource {
        id: format!("source:{key}"),
        provider: if locale.is_some() {
            "ddragon"
        } else {
            "riot_catalog"
        }
        .into(),
        key: key.into(),
        version: "16.19.1".into(),
        locale: locale.map(str::to_owned),
        url: format!("https://ddragon.leagueoflegends.com/cdn/16.19.1/data/{key}"),
        observed_at: "2026-10-01T10:00:00Z".into(),
        data,
    }
}

fn items(data: Value) -> CatalogSource {
    source(
        "fr_FR/item.json",
        json!({"type":"item","version":"16.19.1","data":data}),
    )
}

fn project(data: Value) -> Vec<CatalogRecord> {
    normalize("16.19.1", &[items(data)]).unwrap()
}

fn record<'a>(records: &'a [CatalogRecord], kind: &str, id: &str) -> &'a CatalogRecord {
    records
        .iter()
        .find(|r| r.kind == kind && r.id == id)
        .expect("fiche normalisée attendue")
}

#[test]
fn objets_preservent_prix_zero_faux_stats_et_provenance() {
    let records = project(
        json!({"1001":{"name":"Bottes","description":"<mainText>Vitesse<br>+25</mainText>","image":{"full":"1001.png"},"gold":{"base":0,"total":300,"sell":210,"purchasable":false},"stats":{"FlatMovementSpeedMod":25,"PercentAttackSpeedMod":0.15},"maps":{"11":true,"12":false},"tags":["Boots"],"inStore":false}}),
    );
    let item = record(&records, "item", "1001");
    assert_eq!(item.fields["price_base"].value, 0);
    assert_eq!(item.fields["price_total"].unit.as_deref(), Some("gold"));
    assert_eq!(item.fields["purchasable"].value, false);
    assert_eq!(item.fields["maps"].value, json!({"11":true,"12":false}));
    assert_eq!(item.fields["categories"].value, json!(["Boots"]));
    assert_eq!(item.stats["attack_speed"].value, 0.15);
    assert_eq!(item.stats["attack_speed"].unit.as_deref(), Some("ratio"));
    assert_eq!(
        item.fields["price_total"].sources[0].pointer,
        "/data/1001/gold/total"
    );
    assert_eq!(
        item.fields["name"].sources[0].source_id,
        "source:fr_FR/item.json"
    );
    assert_eq!(item.description.as_deref(), Some("Vitesse +25"));
    assert_eq!(
        item.icon.as_deref(),
        Some("https://ddragon.leagueoflegends.com/cdn/16.19.1/img/item/1001.png")
    );
}

#[test]
fn absents_nulls_vides_et_inconnus_restent_distincts() {
    let records = project(
        json!({"1001":{"name":"","gold":{"total":null},"inStore":false,"stats":{"FlatHPPoolMod":0,"FutureMystery":7},"from":[],"future/key":{"x~y":9}}}),
    );
    let item = record(&records, "item", "1001");
    assert_eq!(item.name, "");
    assert!(!item.fields.contains_key("purchasable"));
    assert_eq!(item.fields["price_total"].status, ValueStatus::Missing);
    assert!(item.fields["price_total"].value.is_null());
    assert_eq!(item.fields["builds_from"].value, json!([]));
    assert_eq!(item.stats["health"].value, 0);
    assert!(item
        .coverage
        .unmapped_fields
        .contains(&"source:fr_FR/item.json:/data/1001/stats/FutureMystery".into()));
    assert!(item
        .coverage
        .unmapped_fields
        .contains(&"source:fr_FR/item.json:/data/1001/future~1key/x~0y".into()));
    assert_eq!(
        item.coverage.source_fields,
        item.coverage.normalized_fields + item.coverage.unmapped_fields.len() as u32
    );
}

#[test]
fn types_invalides_ne_deviennent_pas_valeurs_verifiees() {
    let records = project(
        json!({"1001":{"name":"Objet","gold":{"total":"300","purchasable":"false"},"stats":{"FlatPhysicalDamageMod":"40"},"maps":{"11":"true"}}}),
    );
    let item = record(&records, "item", "1001");
    assert_eq!(item.fields["price_total"].status, ValueStatus::Unsupported);
    assert_eq!(item.fields["purchasable"].status, ValueStatus::Unsupported);
    assert_eq!(item.stats["attack_damage"].status, ValueStatus::Unsupported);
    assert!(!item.coverage.issues.is_empty());
}

#[test]
fn recettes_conservent_ids_et_signalent_cycles_et_references_absentes() {
    let records = project(
        json!({"1001":{"name":"A","from":["1002"],"into":["9999"]},"1002":{"name":"B","from":["1001"]}}),
    );
    let first = record(&records, "item", "1001");
    assert_eq!(first.fields["builds_from"].value, json!(["1002"]));
    assert!(first
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("recipe_cycle")));
    assert!(first.coverage.issues.iter().any(|s| s.contains("9999")));
}

#[test]
fn traductions_gardent_identite_et_objets_transformes_distincts() {
    let fr = items(
        json!({"3003":{"name":"Bâton","into":["3040"]},"3040":{"name":"Étreinte","from":["3003"],"gold":{"purchasable":false}}}),
    );
    let en = source(
        "en_US/item.json",
        json!({"type":"item","version":"16.19.1","data":{"3003":{"name":"Staff","into":["3040"]},"3040":{"name":"Embrace","from":["3003"],"gold":{"purchasable":false}}}}),
    );
    let records = normalize("16.19.1", &[fr, en]).unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(records.iter().filter(|r| r.id == "3003").count(), 2);
    assert!(records.iter().all(|r| r.namespace == "standard"));
    assert!(records
        .iter()
        .all(|r| !r.coverage.issues.iter().any(|s| s.contains("recipe_cycle"))));
}

fn champion(key: &str, id: &str) -> Value {
    json!({"type":"champion","version":"16.19.1","data":{id:{"id":id,"key":key,"name":"Ahri","image":{"full":format!("{id}.png")},"stats":{"hp":590,"hpperlevel":104,"attackdamage":53,"attackspeed":0.668},"partype":"Mana","tags":["Mage"],"passive":{"name":"Passif","description":"Soigne","image":{"full":"passive.png"}},"spells":[{"id":"Orb","name":"Orbe","description":"<p>Dégâts</p>","cooldown":[7,6,5],"range":[900],"cost":[50],"vars":[{"key":"a1","coeff":[0.4]}],"image":{"full":"Orb.png"}},{"id":"Fox","name":"Fox","description":"Feu"},{"id":"Charm","name":"Charm","description":"Charme"},{"id":"Rush","name":"Rush","description":"Ruée"}]}}})
}

#[test]
fn champions_competences_et_namespace_classic_restent_distincts() {
    let records = normalize(
        "16.19.1",
        &[
            source("fr_FR/champion/Ahri.json", champion("103", "Ahri")),
            source(
                "fr_FR/mode/classic/champion/Jade_Ahri.json",
                champion("60103", "Jade_Ahri"),
            ),
        ],
    )
    .unwrap();
    assert_eq!(records.len(), 12);
    assert_eq!(
        record(&records, "champion", "103").stats["health"].value,
        590
    );
    assert_eq!(
        record(&records, "champion", "103").stats["health_per_level"].value,
        104
    );
    let classic = record(&records, "champion", "60103");
    assert_eq!(classic.namespace, "classic");
    assert!(classic
        .icon
        .as_ref()
        .unwrap()
        .contains("/img/mode/classic/champion/"));
    let ability = record(&records, "ability", "103:Q");
    assert_eq!(ability.fields["cooldown"].value, json!([7, 6, 5]));
    assert_eq!(ability.fields["cooldown"].unit.as_deref(), Some("seconds"));
    assert!(ability
        .coverage
        .unmapped_fields
        .iter()
        .any(|s| s.contains("/vars/")));
}

#[test]
fn infobulle_de_competence_expose_les_segments_types_a_cote_du_texte_brut() {
    let mut data = champion("103", "Ahri");
    data["data"]["Ahri"]["spells"][0]["tooltip"] =
        json!("Inflige <magicDamage>{{ e1 }} dégâts magiques</magicDamage> puis <trueDamage>{{ e2 }}</trueDamage>.");
    let records = normalize("16.19.1", &[source("fr_FR/champion/Ahri.json", data)]).unwrap();
    let ability = record(&records, "ability", "103:Q");
    assert_eq!(
        ability.fields["tooltip"].value,
        "Inflige {{ e1 }} dégâts magiques puis {{ e2 }} ."
    );
    let segments = &ability.fields["tooltip_segments"];
    assert_eq!(
        segments.value,
        json!([
            {"text":"Inflige ","damage_type":null},
            {"text":"{{ e1 }} dégâts magiques","damage_type":"magic"},
            {"text":" puis ","damage_type":null},
            {"text":"{{ e2 }}","damage_type":"true"},
            {"text":" .","damage_type":null},
        ])
    );
    assert_eq!(segments.status, ValueStatus::Derived);
    assert_eq!(segments.unit, None);
    assert_eq!(segments.sources.len(), 1);
    assert_eq!(
        segments.sources[0].source_id,
        "source:fr_FR/champion/Ahri.json"
    );
    assert_eq!(segments.sources[0].pointer, "/data/Ahri/spells/0/tooltip");
    // Pas d'infobulle : pas de segments inventés, et le passif n'en porte pas non plus.
    assert!(!record(&records, "ability", "103:W")
        .fields
        .contains_key("tooltip_segments"));
    assert!(!record(&records, "ability", "103:passive")
        .fields
        .contains_key("tooltip_segments"));
    assert!(!ability
        .coverage
        .unmapped_fields
        .iter()
        .any(|s| s.ends_with("/tooltip")));
}

#[test]
fn index_et_fiche_champion_ne_creent_pas_doublon() {
    let details = champion("103", "Ahri");
    let index = source(
        "fr_FR/champion.json",
        json!({"type":"champion","version":"16.19.1","data":{"Ahri":{"id":"Ahri","key":"103","name":"Ahri","stats":{"hp":590}}}}),
    );
    let records = normalize(
        "16.19.1",
        &[index, source("fr_FR/champion/Ahri.json", details)],
    )
    .unwrap();
    assert_eq!(records.iter().filter(|r| r.kind == "champion").count(), 1);
    assert!(
        record(&records, "champion", "103").stats["health"]
            .sources
            .len()
            >= 2
    );
}

#[test]
fn runes_sorts_cartes_icones_et_catalogues_globaux_sont_normalises() {
    let records = normalize("16.19.1", &[
        source("fr_FR/runesReforged.json",json!([{"id":8000,"key":"Precision","name":"Précision","icon":"perk-images/style.png","slots":[{"runes":[{"id":8005,"key":"Press","name":"Attaque","icon":"perk-images/rune.png","shortDesc":"<b>Court</b>","longDesc":"Long"}]}]}])),
        source("fr_FR/summoner.json",json!({"type":"summoner","version":"16.19.1","data":{"SummonerFlash":{"id":"SummonerFlash","key":"4","name":"Saut éclair","cooldown":[300],"modes":["CLASSIC"],"summonerLevel":7}}})),
        source("fr_FR/map.json",json!({"type":"map","version":"16.19.1","data":{"11":{"MapId":"11","MapName":"Faille","image":{"full":"map11.png"}}}})),
        source("fr_FR/profileicon.json",json!({"type":"profileicon","version":"16.19.1","data":{"0":{"id":0,"image":{"full":"0.png"}}}})),
        source("queues",json!([{"queueId":420,"map":"Summoner's Rift","description":"Ranked","notes":null}])),
        source("maps",json!([{"mapId":11,"mapName":"Summoner's Rift","notes":"Current"}])),
        source("gameModes",json!([{"gameMode":"CLASSIC","description":"Classic"}])),
        source("gameTypes",json!([{"gametype":"MATCHED_GAME","description":"Matchmade"}]))
    ]).unwrap();
    assert_eq!(
        record(&records, "rune", "8005").fields["style_id"].value,
        "8000"
    );
    assert_eq!(record(&records, "rune", "8005").fields["slot"].value, 0);
    assert_eq!(
        record(&records, "summoner_spell", "4").fields["cooldown"].value,
        json!([300])
    );
    assert_eq!(record(&records, "queue", "420").locale, "und");
    assert!(records.iter().any(|r| r.kind == "map" && r.locale == "und"));
    assert!(records.iter().any(|r| r.kind == "mode"));
    assert!(records.iter().any(|r| r.kind == "game_type"));
    assert!(record(&records, "profile_icon", "0").icon.is_some());
}

#[test]
fn html_et_urls_dangereuses_ne_traversent_pas_la_projection() {
    let records = project(
        json!({"1001":{"name":"Objet","description":"<p>Vie &amp; dégâts</p><script>alert(1)</script><style>body{}</style><br>&lt;img src=x onerror=bad&gt; +10","image":{"full":"../bad.png"}}}),
    );
    let item = record(&records, "item", "1001");
    let description = item.description.as_ref().unwrap();
    assert!(!description.contains("alert"));
    assert!(!description.contains("body{"));
    assert!(!description.contains('<'));
    assert!(description.contains("Vie & dégâts"));
    assert!(item.icon.is_none());
    assert!(item
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("invalid_icon")));
}

#[test]
fn sources_incoherentes_et_identites_invalides_sont_refusees() {
    let mut wrong = items(json!({"1001":{"name":"Objet"}}));
    wrong.version = "16.18.1".into();
    assert!(normalize("16.19.1", &[wrong]).is_err());
    assert!(normalize("../../bad", &[items(json!({"1001":{"name":"Objet"}}))]).is_err());
    assert!(normalize("16.19.1", &[items(json!({"x":{"name":"Objet"}}))]).is_err());
    assert!(normalize("16.19.1", &[]).is_err());
}

#[test]
fn discordance_locale_et_identite_manquante_sont_signalees() {
    let records = normalize("16.19.1", &[
        items(json!({"1001":{"name":"Objet","gold":{"total":100}},"1002":{"name":"Exclusif"}})),
        source("en_US/item.json",json!({"type":"item","version":"16.19.1","data":{"1001":{"name":"Item","gold":{"total":200}}}}))
    ]).unwrap();
    assert!(record(&records, "item", "1001")
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("locale_conflict:price_total")));
    assert!(record(&records, "item", "1002")
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("missing_locale:en_US")));
}

#[test]
fn couverture_ne_compte_pas_deux_fois_les_champs_des_competences() {
    let data = json!({"type":"champion","version":"16.19.1","data":{"Ahri":{"id":"Ahri","key":"103","name":"Ahri","passive":{"name":"Passif","description":"Passif"},"spells":[{"id":"Q","name":"Q"},{"id":"W","name":"W"},{"id":"E","name":"E"},{"id":"R","name":"R"}]}}});
    let records = normalize("16.19.1", &[source("fr_FR/champion/Ahri.json", data)]).unwrap();
    // Treize feuilles d'entités et deux métadonnées de source, comptées une seule fois.
    assert_eq!(
        records
            .iter()
            .map(|r| r.coverage.source_fields)
            .sum::<u32>(),
        15
    );
    assert_eq!(
        record(&records, "champion", "103").coverage.source_fields,
        5
    );
}

#[test]
fn libelle_de_ressource_localise_ne_devient_pas_conflit_et_icone_zero_est_valide() {
    let mut fr = champion("103", "Ahri");
    fr["data"]["Ahri"]["partype"] = json!("Énergie");
    let mut en = fr.clone();
    en["data"]["Ahri"]["partype"] = json!("Energy");
    let records=normalize("16.19.1", &[
        source("fr_FR/champion/Ahri.json",fr),source("en_US/champion/Ahri.json",en),
        source("fr_FR/profileicon.json",json!({"type":"profileicon","version":"16.19.1","data":{"0":{"id":0,"image":{"full":"0.png"}}}}))
    ]).unwrap();
    assert!(!record(&records, "champion", "103")
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("locale_conflict:resource")));
    assert!(record(&records, "profile_icon", "0")
        .coverage
        .issues
        .is_empty());
}

#[test]
fn details_mal_adresses_et_ids_champions_dupliques_sont_refuses() {
    assert!(normalize(
        "16.19.1",
        &[source("fr_FR/champion/Garen.json", champion("103", "Ahri"))]
    )
    .is_err());
    let index = json!({"type":"champion","version":"16.19.1","data":{"Ahri":{"id":"Ahri","key":"103","name":"Ahri"},"Garen":{"id":"Garen","key":"103","name":"Garen"}}});
    assert!(normalize("16.19.1", &[source("fr_FR/champion.json", index)]).is_err());
}

#[test]
fn recette_avec_lien_inverse_contradictoire_est_signalee() {
    let records = project(
        json!({"1001":{"name":"Composant","into":[]},"1002":{"name":"Objet","from":["1001"]}}),
    );
    assert!(record(&records, "item", "1002")
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("inconsistent_recipe:1001")));
}

#[test]
fn couverture_inclut_enveloppe_et_metadonnees_non_interpretees_une_seule_fois() {
    let source = source(
        "fr_FR/item.json",
        json!({"type":"item","version":"16.19.1","format":"standAloneComplex","basic":{"future":42},"data":{"1001":{"name":"A"},"1002":{"name":"B"}}}),
    );
    let records = normalize("16.19.1", &[source]).unwrap();
    assert_eq!(
        records
            .iter()
            .map(|r| r.coverage.source_fields)
            .sum::<u32>(),
        6
    );
    assert_eq!(
        records
            .iter()
            .map(|r| r.coverage.normalized_fields)
            .sum::<u32>(),
        4
    );
    let unmapped: Vec<_> = records
        .iter()
        .flat_map(|r| r.coverage.unmapped_fields.iter())
        .collect();
    assert_eq!(unmapped.len(), 2);
    assert!(unmapped
        .iter()
        .any(|p| *p == "source:fr_FR/item.json:/basic/future"));
    assert!(unmapped
        .iter()
        .any(|p| *p == "source:fr_FR/item.json:/format"));
}

#[test]
fn retraitement_est_independant_de_l_ordre_des_sources() {
    let index = source(
        "fr_FR/champion.json",
        json!({"type":"champion","version":"16.19.1","data":{"Ahri":{"id":"Ahri","key":"103","name":"Ahri","stats":{"hp":590}}}}),
    );
    let detail = source("fr_FR/champion/Ahri.json", champion("103", "Ahri"));
    assert_eq!(
        normalize("16.19.1", &[index.clone(), detail.clone()]).unwrap(),
        normalize("16.19.1", &[detail, index]).unwrap()
    );
}

#[test]
fn enrichissement_revalide_relations_et_retire_les_anciens_inconnus() {
    let mut records = project(json!({"1001":{"name":"A","into":["1002"]}}));
    assert!(record(&records, "item", "1001")
        .coverage
        .issues
        .iter()
        .any(|s| s.contains("unknown_recipe_reference:1002")));
    let mut extra = project(json!({"1002":{"name":"B","from":["1001"]}}));
    records.append(&mut extra);
    finalize(&mut records);
    assert!(!records.iter().any(|r| r
        .coverage
        .issues
        .iter()
        .any(|s| s.starts_with("unknown_recipe_reference:"))));
    // Une relation supplémentaire introduite par le complément crée ici un cycle.
    let mut value = records[0].fields["builds_into"].clone();
    value.value = json!(["1002"]);
    records[0].fields.insert("builds_from".into(), value);
    finalize(&mut records);
    assert!(records
        .iter()
        .all(|r| r.coverage.issues.iter().any(|s| s == "recipe_cycle")));
    let previous = records.clone();
    finalize(&mut records);
    assert_eq!(records, previous);
}

#[test]
fn regeneration_objets_et_champions_gardent_leurs_unites_sources_distinctes() {
    let mut ahri = champion("103", "Ahri");
    ahri["data"]["Ahri"]["stats"]["hpregen"] = json!(2.5);
    ahri["data"]["Ahri"]["stats"]["hpregenperlevel"] = json!(0.6);
    let records = normalize(
        "16.19.1",
        &[
            items(json!({"1054":{"name":"Bouclier","stats":{"FlatHPRegenMod":0.8}}})),
            source("fr_FR/champion/Ahri.json", ahri),
        ],
    )
    .unwrap();
    let item = record(&records, "item", "1054");
    assert_eq!(item.stats["health_regeneration"].value, 0.8);
    assert_eq!(
        item.stats["health_regeneration"].unit.as_deref(),
        Some("points_per_second")
    );
    let champion = record(&records, "champion", "103");
    assert_eq!(champion.stats["health_regeneration"].value, 2.5);
    assert_eq!(
        champion.stats["health_regeneration"].unit.as_deref(),
        Some("points_per_5_seconds")
    );
    assert_eq!(
        champion.stats["health_regeneration_per_level"]
            .unit
            .as_deref(),
        Some("points_per_5_seconds_per_level")
    );
}

#[test]
fn regeneration_mana_conserve_les_valeurs_et_les_periodes_sources() {
    let mut ahri = champion("103", "Ahri");
    ahri["data"]["Ahri"]["stats"]["mpregen"] = json!(8.0);
    ahri["data"]["Ahri"]["stats"]["mpregenperlevel"] = json!(0.8);
    let records = normalize(
        "16.19.1",
        &[
            items(json!({"773069":{"name":"Objet","stats":{"FlatMPRegenMod":2.0}}})),
            source("fr_FR/champion/Ahri.json", ahri),
        ],
    )
    .unwrap();
    let item = record(&records, "item", "773069");
    assert_eq!(item.stats["mana_regeneration"].value, 2.0);
    assert_eq!(item.stats["mana_regeneration"].unit, None);
    let champion = record(&records, "champion", "103");
    assert_eq!(champion.stats["mana_regeneration"].value, 8.0);
    assert_eq!(
        champion.stats["mana_regeneration"].unit.as_deref(),
        Some("points_per_5_seconds")
    );
    assert_eq!(champion.stats["mana_regeneration_per_level"].value, 0.8);
    assert_eq!(
        champion.stats["mana_regeneration_per_level"]
            .unit
            .as_deref(),
        Some("points_per_5_seconds_per_level")
    );
}

#[test]
fn couverture_attribue_les_pointeurs_ambigus_a_chaque_source() {
    let mut details = champion("103", "Ahri");
    details["data"]["Ahri"]["future"] = json!(2);
    let index = source(
        "fr_FR/champion.json",
        json!({"type":"champion","version":"16.19.1","data":{"Ahri":{"id":"Ahri","key":"103","name":"Ahri","future":1}}}),
    );
    let records = normalize(
        "16.19.1",
        &[index, source("fr_FR/champion/Ahri.json", details)],
    )
    .unwrap();
    let missing = &record(&records, "champion", "103").coverage.unmapped_fields;
    assert!(missing.contains(&"source:fr_FR/champion.json:/data/Ahri/future".into()));
    assert!(missing.contains(&"source:fr_FR/champion/Ahri.json:/data/Ahri/future".into()));
}
