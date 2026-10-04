use super::*;

fn seg(text: &str, damage_type: Option<DamageType>) -> TooltipSegment {
    TooltipSegment {
        text: text.into(),
        damage_type,
    }
}

fn joined(segments: &[TooltipSegment]) -> String {
    segments.iter().map(|s| s.text.as_str()).collect()
}

/// Infobulle réelle de l'Orbe d'Ahri (Data Dragon 16.19.1, en_US) : les balises de type de dégâts
/// existent bien dans la source, ce que le ticket #107 donnait pour plausible mais non établi.
const AHRI_Q: &str = "Ahri throws then pulls back her orb, dealing <magicDamage>{{ totaldamage }} magic damage</magicDamage> on the way out and <trueDamage>{{ totaldamage }} true damage</trueDamage> on the way back.{{ spellmodifierdescriptionappend }}";

#[test]
fn segments_typent_les_degats_depuis_les_balises_de_la_source() {
    assert_eq!(
        tooltip_segments(AHRI_Q),
        vec![
            seg("Ahri throws then pulls back her orb, dealing ", None),
            seg("{{ totaldamage }} magic damage", Some(DamageType::Magic)),
            seg(" on the way out and ", None),
            seg("{{ totaldamage }} true damage", Some(DamageType::True)),
            seg(
                " on the way back.{{ spellmodifierdescriptionappend }}",
                None
            ),
        ]
    );
}

#[test]
fn le_type_ne_depend_pas_de_la_langue_du_texte() {
    let segments = tooltip_segments(
        "Inflige <physicalDamage>{{ e1 }} points de dégâts physiques</physicalDamage>.",
    );
    assert_eq!(
        segments,
        vec![
            seg("Inflige ", None),
            seg(
                "{{ e1 }} points de dégâts physiques",
                Some(DamageType::Physical)
            ),
            seg(" .", None),
        ]
    );
}

#[test]
fn les_balises_imbriquees_heritent_du_type_et_les_segments_voisins_fusionnent() {
    let segments =
        tooltip_segments("<magicDamage>10 <scaleAP>(+ 50 % AP)</scaleAP> dégâts</magicDamage>");
    assert_eq!(
        segments,
        vec![seg("10 (+ 50 % AP) dégâts", Some(DamageType::Magic))]
    );
}

#[test]
fn le_type_de_degats_le_plus_interne_l_emporte() {
    let segments =
        tooltip_segments("<physicalDamage>a <trueDamage>b</trueDamage> c</physicalDamage>");
    assert_eq!(
        segments,
        vec![
            seg("a ", Some(DamageType::Physical)),
            seg("b", Some(DamageType::True)),
            seg(" c", Some(DamageType::Physical)),
        ]
    );
}

#[test]
fn balises_inconnues_non_fermees_ou_orphelines_ne_cassent_pas_le_typage() {
    // Balises sans type de dégâts : texte simple.
    assert_eq!(
        tooltip_segments("<status>Charme</status><br /><speed>+20 %</speed>"),
        vec![seg("Charme +20 %", None)]
    );
    // Balise de dégâts jamais fermée : le type court jusqu'à la fin du texte.
    assert_eq!(
        tooltip_segments("x <MagicDamage>50"),
        vec![seg("x ", None), seg("50", Some(DamageType::Magic))]
    );
    // Fermeture orpheline : ignorée, sans effet sur le texte suivant.
    assert_eq!(
        tooltip_segments("a </magicDamage> b"),
        vec![seg("a b", None)]
    );
}

#[test]
fn entrees_vides_ou_sans_balise() {
    assert!(tooltip_segments("").is_empty());
    assert!(tooltip_segments("  <br />  ").is_empty());
    assert_eq!(tooltip_segments("Rien"), vec![seg("Rien", None)]);
}

#[test]
fn le_texte_des_segments_est_toujours_celui_de_plain_text() {
    for input in [
        AHRI_Q,
        "",
        "Rien",
        "a<br />b<br/>c<br>d",
        "<magicDamage> 50 </magicDamage>.",
        "<magicDamage>a</magicDamage><magicDamage>b</magicDamage>",
        "&lt;magicDamage&gt;50&lt;/magicDamage&gt; &amp; &#233;&nbsp;x",
        "<script>alert(1)</script>avant<style>body{}</style>après",
        "a < b > c <magicDamage",
        "<physicalDamage>a <trueDamage>b</trueDamage> c</physicalDamage> d",
        "x </magicDamage> y <unknown attr=\"1\">z</unknown>",
    ] {
        assert_eq!(
            joined(&tooltip_segments(input)),
            plain_text(input),
            "{input}"
        );
    }
}

#[test]
fn le_contenu_actif_ne_traverse_pas_les_segments() {
    let segments = tooltip_segments("<script>alert(1)</script><magicDamage>ok</magicDamage>");
    assert_eq!(segments, vec![seg("ok", Some(DamageType::Magic))]);
}
