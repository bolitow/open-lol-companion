//! Étapes d'achat (#81) : départ, bottes, core et emplacements 4 à 6, jointes au
//! catalogue normalisé (#61) du patch de la partie. Chaque étape est une catégorie
//! de build indépendante, avec sa propre population.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) use super::builds::Purchase;

/// Achats strictement avant 1 min 30 : la timeline match-v5 ne signale ni retour ni
/// sortie de base, cette fenêtre approxime les achats faits avant de quitter la fontaine.
pub(super) const STARTER_WINDOW_MS: u64 = 90_000;
/// Prix total minimal d'un objet complet : exclut objets de départ, de quête et Mejai.
pub(super) const COMPLETED_MIN_PRICE: u64 = 2_000;
/// Bottes de base (achetées ou offertes par une rune) : jamais une étape « bottes ».
const BASE_BOOTS: [u32; 2] = [1001, 2422];
/// Borne défensive des chaînes `special_recipe` / `builds_from` du catalogue.
const MAX_CHAIN: usize = 8;

pub(super) const STAGE_CATEGORIES: [&str; 6] = [
    "starter",
    "boots",
    "core",
    "item_slot_4",
    "item_slot_5",
    "item_slot_6",
];

pub(super) const STAGE_METHOD: &str = "catalog #61 of the game patch; starter = net purchases \
before 90000 ms without trinkets (sorted multiset); boots = first net purchase whose builds_from \
chain reaches 1001/2422 or tagged Boots, empty if none; completed = purchasable, in store, no \
builds_into, price_total >= 2000, not Consumable/Trinket/Boots, transformations folded via \
special_recipe; core = first 3 distinct completed items in purchase order; item_slot_N = Nth";

#[derive(Debug, Default)]
struct Item {
    price_total: Option<u64>,
    purchasable: Option<bool>,
    in_store: Option<bool>,
    builds_from: Vec<u32>,
    has_builds_into: bool,
    tags: Vec<String>,
    special_recipe: Option<u32>,
}

/// Classement des objets d'un patch, calculé une fois et réutilisé pour chaque partie.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemCatalog {
    pub version: String,
    completed: BTreeSet<u32>,
    boots: BTreeSet<u32>,
    trinkets: BTreeSet<u32>,
    canonical: BTreeMap<u32, u32>,
}

impl ItemCatalog {
    /// `records` : identifiant et `data.fields` d'une fiche `item` normalisée.
    /// Seules les valeurs `verified` ou `derived` sont lues ; le reste n'est pas deviné.
    pub(super) fn from_records<'a>(
        version: &str,
        records: impl IntoIterator<Item = (&'a str, &'a Value)>,
    ) -> Self {
        let mut items = BTreeMap::new();
        for (id, fields) in records {
            let Ok(id) = id.parse::<u32>() else { continue };
            items.insert(
                id,
                Item {
                    price_total: field(fields, "price_total").and_then(Value::as_u64),
                    purchasable: field(fields, "purchasable").and_then(Value::as_bool),
                    in_store: field(fields, "in_store").and_then(Value::as_bool),
                    builds_from: ids(field(fields, "builds_from")),
                    has_builds_into: !ids(field(fields, "builds_into")).is_empty(),
                    tags: field(fields, "categories")
                        .and_then(Value::as_array)
                        .map(|tags| {
                            tags.iter()
                                .filter_map(|t| t.as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default(),
                    special_recipe: field(fields, "special_recipe")
                        .and_then(Value::as_u64)
                        .and_then(|v| u32::try_from(v).ok())
                        .filter(|v| *v > 0),
                },
            );
        }
        let mut catalog = Self {
            version: version.into(),
            ..Self::default()
        };
        for (id, item) in &items {
            let mut root = *id;
            for _ in 0..MAX_CHAIN {
                match items.get(&root).and_then(|i| i.special_recipe) {
                    Some(next) if next != *id => root = next,
                    _ => break,
                }
            }
            if root != *id {
                catalog.canonical.insert(*id, root);
            }
            if item.tags.iter().any(|t| t == "Trinket") {
                catalog.trinkets.insert(*id);
            }
            if boots_chain(&items, *id, 0) {
                catalog.boots.insert(*id);
            }
        }
        for (id, item) in &items {
            // DDragon omet `inStore` quand il vaut sa valeur par défaut (true).
            let completed = item.purchasable == Some(true)
                && item.in_store != Some(false)
                && !item.has_builds_into
                && item.price_total.is_some_and(|p| p >= COMPLETED_MIN_PRICE)
                && !item
                    .tags
                    .iter()
                    .any(|t| matches!(t.as_str(), "Consumable" | "Trinket" | "Boots"))
                && !catalog.boots.contains(id);
            if completed {
                catalog.completed.insert(*id);
            }
        }
        catalog
    }

    /// Empreinte du classement (#89) : version et ensembles qui décident des étapes. Une
    /// même version republiée avec d'autres fiches (`catalog --refresh` ou `--rebuild`,
    /// complément CommunityDragon) change d'empreinte dès qu'une étape peut changer ; une
    /// republication sans effet sur le classement la garde. Déstructuration exhaustive,
    /// sans `..` : un champ ajouté au catalogue ne compile plus tant qu'il n'est pas ici.
    pub(super) fn fingerprint(&self) -> String {
        let Self {
            version,
            completed,
            boots,
            trinkets,
            canonical,
        } = self;
        let text = format!("{version}\n{completed:?}\n{boots:?}\n{trinkets:?}\n{canonical:?}");
        format!("{:x}", Sha256::digest(text.as_bytes()))
    }

    /// Objet d'origine d'une transformation (Muramana → Manamune), sinon l'objet lui-même.
    pub(super) fn canonical(&self, item: u32) -> u32 {
        self.canonical.get(&item).copied().unwrap_or(item)
    }

    pub(super) fn is_completed(&self, item: u32) -> bool {
        self.completed.contains(&item)
    }

    pub(super) fn is_boots(&self, item: u32) -> bool {
        self.boots.contains(&item)
    }

    /// Étapes d'une séquence nette d'achats, déjà triée par horodatage.
    pub(super) fn derive_steps(&self, purchases: &[Purchase]) -> BTreeMap<String, Vec<u32>> {
        let mut steps = BTreeMap::new();
        let mut starter: Vec<u32> = purchases
            .iter()
            .filter(|p| p.timestamp_ms < STARTER_WINDOW_MS && !self.trinkets.contains(&p.item_id))
            .map(|p| p.item_id)
            .collect();
        starter.sort_unstable();
        steps.insert("starter".into(), starter);
        let boots = purchases.iter().find(|p| self.is_boots(p.item_id));
        steps.insert(
            "boots".into(),
            boots.map(|p| vec![p.item_id]).unwrap_or_default(),
        );
        let mut completed = Vec::new();
        for purchase in purchases {
            let item = self.canonical(purchase.item_id);
            if self.is_completed(item) && !completed.contains(&item) {
                completed.push(item);
            }
        }
        if completed.len() >= 3 {
            steps.insert("core".into(), completed[..3].to_vec());
        }
        for (slot, item) in completed.iter().enumerate().skip(3).take(3) {
            steps.insert(format!("item_slot_{}", slot + 1), vec![*item]);
        }
        steps
    }
}

fn field<'a>(fields: &'a Value, name: &str) -> Option<&'a Value> {
    let entry = fields.get(name)?;
    matches!(entry["status"].as_str(), Some("verified" | "derived")).then(|| &entry["value"])
}

fn ids(value: Option<&Value>) -> Vec<u32> {
    value
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(|id| {
                    id.as_str()
                        .and_then(|s| s.parse().ok())
                        .or_else(|| id.as_u64().and_then(|n| u32::try_from(n).ok()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Bottes : étiquette Boots, ou chaîne `builds_from` remontant à des bottes de base.
/// L'étiquette manque sur certaines bottes de niveau 3 (Gunmetal Greaves, 3172).
fn boots_chain(items: &BTreeMap<u32, Item>, id: u32, depth: usize) -> bool {
    if BASE_BOOTS.contains(&id) || depth > MAX_CHAIN {
        return false;
    }
    let Some(item) = items.get(&id) else {
        return false;
    };
    item.tags.iter().any(|t| t == "Boots")
        || item
            .builds_from
            .iter()
            .any(|from| BASE_BOOTS.contains(from) || boots_chain(items, *from, depth + 1))
}

#[cfg(test)]
#[path = "stages_tests.rs"]
mod tests;
