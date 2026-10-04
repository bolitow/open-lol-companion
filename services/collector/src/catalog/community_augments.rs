//! Augments Arena et Mayhem (#118) : catalogue statique seulement (identité, noms FR/EN,
//! description, rareté, icône, modes qui les listent). Aucune mesure de performance ni de popularité.
//!
//! Sources CommunityDragon, formes relevées sur le patch 16.19 :
//! - `…/v1/cherry-augments.json` (par langue) : `id`, `augmentNameId`, `nameTRA`,
//!   `simpleNameTRA`, `augmentSmallIconPath`, `rarity` ; aucune description n'y figure ;
//! - `…/v1/augment-lists.json` : `{modeName, augmentList}` où chaque entrée est
//!   `Maps/ModeSpecificData/Augments/{augmentNameId}` ;
//! - `cdragon/arena/{fr_fr,en_us}.json` (export généré, hors du schéma `plugins/…`) :
//!   `{augments: [{id, apiName, name, desc, tooltip, rarity, iconSmall, iconLarge, dataValues,
//!   calculations}]}` pour 225 augments seulement. `id` et `apiName` correspondent à `id` et
//!   `augmentNameId` de cherry-augments ; seule `desc` est reprise. Les autres augments, Mayhem,
//!   n'ont aucune description publiée : `missing:description`.
use super::*;

type Sources<'a> = BTreeMap<&'a str, &'a CatalogSource>;
/// Une liste de modes : `(modeName, index dans la source, augmentNameId contenus)`.
type ModeList<'a> = (&'a str, usize, BTreeSet<&'a str>);

const LISTS: &str = "augment-lists.json";
/// Clé racine de l'export Arena (liste d'augments de même `id` et `apiName` que cherry-augments).
const ARENA_ROOT: &str = "augments";
const LIST_PREFIX: &str = "Maps/ModeSpecificData/Augments/";
/// Raretés observées dans l'export ; toute autre valeur reste visible mais non interprétée.
const RARITIES: [&str; 4] = ["kSilver", "kGold", "kPrismatic", "kEventChoice"];

fn locale_key(locale: &str) -> String {
    format!("{locale}/cherry-augments.json")
}

fn arena_key(locale: &str) -> String {
    format!("{locale}/arena-augments.json")
}

/// Export Arena par `id` : `(index dans la source, apiName, fiche)`. `desc` doit être du texte ;
/// `id` et `apiName` sont uniques, sinon la source est refusée.
fn arena_index(data: &Value) -> Result<BTreeMap<u64, (usize, &str, &Value)>, CatalogError> {
    let list = data
        .get(ARENA_ROOT)
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty())
        .ok_or(CatalogError::InvalidSource)?;
    let mut result = BTreeMap::new();
    let mut technical = BTreeSet::new();
    for (position, augment) in list.iter().enumerate() {
        let id = augment
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(CatalogError::InvalidSource)?;
        let name = augment
            .get("apiName")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or(CatalogError::InvalidSource)?;
        if augment.get("desc").and_then(Value::as_str).is_none()
            || !technical.insert(name)
            || result.insert(id, (position, name, augment)).is_some()
        {
            return Err(CatalogError::InvalidSource);
        }
    }
    Ok(result)
}

/// `id` numérique, identité technique et nom textuels, uniques ; sinon la source est refusée.
fn index(data: &Value) -> Result<BTreeMap<u64, (usize, &Value)>, CatalogError> {
    let list = data
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or(CatalogError::InvalidSource)?;
    let mut result = BTreeMap::new();
    let mut technical = BTreeSet::new();
    for (position, augment) in list.iter().enumerate() {
        let id = augment
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(CatalogError::InvalidSource)?;
        let name = augment
            .get("augmentNameId")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or(CatalogError::InvalidSource)?;
        if augment.get("nameTRA").and_then(Value::as_str).is_none()
            || !technical.insert(name)
            || result.insert(id, (position, augment)).is_some()
        {
            return Err(CatalogError::InvalidSource);
        }
    }
    Ok(result)
}

/// Modes déclarés par la source, dans son ordre.
fn lists(data: &Value) -> Result<Vec<ModeList<'_>>, CatalogError> {
    let modes = data
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or(CatalogError::InvalidSource)?;
    let mut result: Vec<ModeList<'_>> = vec![];
    for (position, mode) in modes.iter().enumerate() {
        let name = mode
            .get("modeName")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or(CatalogError::InvalidSource)?;
        let entries = mode
            .get("augmentList")
            .and_then(Value::as_array)
            .ok_or(CatalogError::InvalidSource)?;
        let mut members = BTreeSet::new();
        for entry in entries {
            let technical = entry
                .as_str()
                .and_then(|path| path.strip_prefix(LIST_PREFIX))
                .filter(|name| !name.is_empty() && !name.contains('/'))
                .ok_or(CatalogError::InvalidSource)?;
            if !members.insert(technical) {
                return Err(CatalogError::InvalidSource);
            }
        }
        if result.iter().any(|(known, _, _)| *known == name) {
            return Err(CatalogError::InvalidSource);
        }
        result.push((name, position, members));
    }
    Ok(result)
}

/// Mêmes identifiants et mêmes identités techniques en FR et EN (fiches et export Arena), export
/// Arena rattaché à des fiches existantes, listes de modes résolues.
pub(super) fn validate(sources: &Sources<'_>) -> Result<(), CatalogError> {
    let fr = index(&sources[locale_key("fr_FR").as_str()].data)?;
    let en = index(&sources[locale_key("en_US").as_str()].data)?;
    if fr.keys().ne(en.keys())
        || fr
            .iter()
            .any(|(id, (_, augment))| augment["augmentNameId"] != en[id].1["augmentNameId"])
    {
        return Err(CatalogError::InvalidSource);
    }
    let arena_fr = arena_index(&sources[arena_key("fr_FR").as_str()].data)?;
    let arena_en = arena_index(&sources[arena_key("en_US").as_str()].data)?;
    if arena_fr.len() != arena_en.len()
        || arena_fr.iter().any(|(id, (_, api_name, _))| {
            arena_en.get(id).map(|(_, name, _)| name) != Some(api_name)
        })
        // Chaque augment de l'export doit être une fiche connue, de même identité technique.
        || arena_en
            .iter()
            .any(|(id, (_, api_name, _))| en.get(id).map(|(_, a)| &a["augmentNameId"]) != Some(&json!(api_name)))
    {
        return Err(CatalogError::InvalidSource);
    }
    let known: BTreeSet<_> = en
        .values()
        .filter_map(|(_, augment)| augment["augmentNameId"].as_str())
        .collect();
    // Une liste qui cite un augment absent des fiches serait une référence cassée.
    if lists(&sources[LISTS].data)?
        .iter()
        .any(|(_, _, members)| members.iter().any(|name| !known.contains(name)))
    {
        return Err(CatalogError::InvalidSource);
    }
    Ok(())
}

pub(super) fn enrich(version: &str, records: &mut Vec<CatalogRecord>, sources: &Sources<'_>) {
    let lists_source = sources[LISTS];
    // Les structures et identités ont été validées avant toute mutation.
    let modes = lists(&lists_source.data).unwrap_or_default();
    for locale in ["fr_FR", "en_US"] {
        let source = sources[locale_key(locale).as_str()];
        let arena_source = sources[arena_key(locale).as_str()];
        let arena = arena_index(&arena_source.data).unwrap_or_default();
        for (id, (position, augment)) in index(&source.data).unwrap_or_default() {
            let id_number = id;
            let id = id.to_string();
            let pointer = format!("/{position}");
            let name = plain_text(augment["nameTRA"].as_str().unwrap_or_default());
            let pos = records.iter().position(|r| {
                r.kind == "augment" && r.id == id && r.locale == locale && r.namespace == "standard"
            });
            let pos = pos.unwrap_or_else(|| {
                records.push(CatalogRecord {
                    kind: "augment".into(),
                    id: id.clone(),
                    namespace: "standard".into(),
                    locale: locale.into(),
                    name: if name.is_empty() {
                        id.clone()
                    } else {
                        name.clone()
                    },
                    description: None,
                    icon: None,
                    fields: BTreeMap::new(),
                    stats: BTreeMap::new(),
                    effects: vec![],
                    coverage: RecordCoverage::default(),
                });
                records.len() - 1
            });
            let record = &mut records[pos];
            let mut mapped: BTreeSet<_> = ["id", "augmentNameId", "nameTRA", "simpleNameTRA"]
                .map(|key| format!("{pointer}/{key}"))
                .into_iter()
                .collect();
            if name.is_empty() {
                record.coverage.issues.push("missing:community_name".into());
            }
            add_field(
                record,
                "community_name",
                observed(
                    source,
                    format!("{pointer}/nameTRA"),
                    json!(name),
                    None,
                    ValueStatus::Descriptive,
                ),
            );
            add_field(
                record,
                "community_id",
                observed(
                    source,
                    format!("{pointer}/id"),
                    json!(id),
                    None,
                    ValueStatus::Derived,
                ),
            );
            add_field(
                record,
                "technical_id",
                observed(
                    source,
                    format!("{pointer}/augmentNameId"),
                    augment["augmentNameId"].clone(),
                    None,
                    ValueStatus::Verified,
                ),
            );
            // Le nom court n'existe que si la source le renseigne ; une chaîne vide est une absence.
            if let Some(short) = augment["simpleNameTRA"].as_str().filter(|s| !s.is_empty()) {
                add_field(
                    record,
                    "community_short_name",
                    observed(
                        source,
                        format!("{pointer}/simpleNameTRA"),
                        json!(plain_text(short)),
                        None,
                        ValueStatus::Descriptive,
                    ),
                );
            }
            let rarity = &augment["rarity"];
            let status = match rarity.as_str() {
                _ if rarity.is_null() => ValueStatus::Missing,
                Some(known) if RARITIES.contains(&known) => ValueStatus::Verified,
                _ => ValueStatus::Unsupported,
            };
            if status == ValueStatus::Verified {
                mapped.insert(format!("{pointer}/rarity"));
            }
            add_field(
                record,
                "rarity",
                observed(
                    source,
                    format!("{pointer}/rarity"),
                    rarity.clone(),
                    None,
                    status,
                ),
            );
            if let Some(path) = augment["augmentSmallIconPath"].as_str() {
                if record.icon.is_none() {
                    record.icon = perks::icon_url(version, path);
                }
                add_field(
                    record,
                    "community_icon_path",
                    observed(
                        source,
                        format!("{pointer}/augmentSmallIconPath"),
                        json!(path),
                        None,
                        ValueStatus::Verified,
                    ),
                );
                mapped.insert(format!("{pointer}/augmentSmallIconPath"));
            }
            let technical = augment["augmentNameId"].as_str().unwrap_or_default();
            let listed: Vec<_> = modes
                .iter()
                .filter(|(_, _, members)| members.contains(technical))
                .collect();
            // Absent de toutes les listes : fait observé (pointeur racine), pas une valeur manquante.
            let origins = if listed.is_empty() {
                vec![ValueSource {
                    source_id: lists_source.id.clone(),
                    pointer: String::new(),
                }]
            } else {
                listed
                    .iter()
                    .map(|(_, list, _)| ValueSource {
                        source_id: lists_source.id.clone(),
                        pointer: format!("/{list}/augmentList"),
                    })
                    .collect()
            };
            add_field(
                record,
                "modes",
                CatalogValue {
                    value: json!(listed.iter().map(|(mode, _, _)| *mode).collect::<Vec<_>>()),
                    unit: None,
                    status: ValueStatus::Derived,
                    sources: origins,
                },
            );
            coverage(record, augment, &pointer, &mapped, source);
            describe(record, arena.get(&id_number), arena_source);
        }
    }
}

/// Description depuis l'export Arena (`desc` en texte brut, placeholders `@…@`, `{{ … }}` et jetons
/// d'icône `%i:…%` laissés non résolus et signalés). Sans fiche, ou avec un texte vide, elle est absente : jamais déduite du nom.
fn describe(
    record: &mut CatalogRecord,
    arena: Option<&(usize, &str, &Value)>,
    source: &CatalogSource,
) {
    let described = arena.and_then(|(position, _, augment)| {
        let text = plain_text(augment["desc"].as_str()?);
        (!text.is_empty()).then_some((*position, *augment, text))
    });
    let Some((position, augment, text)) = described else {
        record.coverage.issues.push("missing:description".into());
        return;
    };
    let pointer = format!("/{ARENA_ROOT}/{position}");
    if record.description.is_none() {
        record.description = Some(text.clone());
    }
    // `%i:Augment%`, `%i:StatAnvil%`… : jetons d'icône du client, que `plain_text` ne retire pas ; comme
    // `@…@` et `{{ … }}` ils restent dans le texte, jamais résolus ni remplacés, et sont signalés.
    if text.contains('@') || text.contains("{{") || text.contains("%i:") {
        record
            .coverage
            .issues
            .push("unresolved_placeholder:description".into());
    }
    add_field(
        record,
        "community_description",
        observed(
            source,
            format!("{pointer}/desc"),
            json!(text),
            None,
            ValueStatus::Descriptive,
        ),
    );
    // `id` et `apiName` ont servi à rattacher la fiche ; `desc` est reprise. Le reste (noms, rareté et
    // icônes déjà lus dans cherry-augments, `tooltip`, `dataValues`, `calculations`) reste non mappé.
    let mapped: BTreeSet<_> = ["id", "apiName", "desc"]
        .map(|key| format!("{pointer}/{key}"))
        .into_iter()
        .collect();
    coverage(record, augment, &pointer, &mapped, source);
}
