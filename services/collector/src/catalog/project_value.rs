//! Valeurs typées, provenance et couverture des champs d'une entité source.
use super::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) enum Shape {
    Text,
    Number,
    Bool,
    Strings,
    Ids,
    Numbers,
    BoolMap,
}

pub(super) fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn leaves(value: &Value, pointer: &str, paths: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) if !object.is_empty() => {
            for (key, child) in object {
                leaves(child, &format!("{pointer}/{}", escape(key)), paths);
            }
        }
        Value::Array(array) if !array.is_empty() => {
            for (index, child) in array.iter().enumerate() {
                leaves(child, &format!("{pointer}/{index}"), paths);
            }
        }
        _ => {
            paths.insert(pointer.into());
        }
    }
}

/// L'enveloppe partagée est comptée une seule fois, sur la première fiche de la source.
/// Les chemins source_id:/basic/… restent attribuables après fusion des documents.
pub(super) fn envelope_coverage(source: &CatalogSource, record: &mut CatalogRecord) {
    let Some(object) = source.data.as_object() else {
        return;
    };
    for (key, value) in object.iter().filter(|(key, _)| key.as_str() != "data") {
        let mut paths = BTreeSet::new();
        leaves(value, &format!("/{}", escape(key)), &mut paths);
        record.coverage.source_fields += paths.len() as u32;
        if matches!(key.as_str(), "type" | "version") {
            record.coverage.normalized_fields += paths.len() as u32;
        } else {
            record.coverage.unmapped_fields.extend(
                paths
                    .into_iter()
                    .map(|pointer| format!("{}:{pointer}", source.id)),
            );
        }
    }
}

pub(super) struct Builder<'a> {
    pub source: &'a CatalogSource,
    pub entry: &'a Value,
    pub pointer: String,
    pub record: CatalogRecord,
    paths: BTreeSet<String>,
    used: BTreeSet<String>,
}

impl<'a> Builder<'a> {
    pub fn new(
        source: &'a CatalogSource,
        entry: &'a Value,
        pointer: String,
        kind: &str,
        id: String,
        namespace: &str,
    ) -> Self {
        let mut paths = BTreeSet::new();
        leaves(entry, &pointer, &mut paths);
        Self {
            source,
            entry,
            pointer,
            paths,
            used: BTreeSet::new(),
            record: CatalogRecord {
                kind: kind.into(),
                id,
                namespace: namespace.into(),
                locale: source.locale.clone().unwrap_or_else(|| "und".into()),
                name: String::new(),
                description: None,
                icon: None,
                fields: BTreeMap::new(),
                stats: BTreeMap::new(),
                effects: Vec::new(),
                coverage: RecordCoverage::default(),
            },
        }
    }

    pub fn consume(&mut self, path: &str) {
        let full = format!("{}{path}", self.pointer);
        let prefix = format!("{full}/");
        self.used.extend(
            self.paths
                .iter()
                .filter(|p| *p == &full || p.starts_with(&prefix))
                .cloned(),
        );
    }

    /// Une sous-entité publiée séparément possède sa propre couverture.
    pub fn delegate(&mut self, path: &str) {
        let full = format!("{}{path}", self.pointer);
        let prefix = format!("{full}/");
        self.paths.retain(|p| p != &full && !p.starts_with(&prefix));
    }

    pub fn origin(&self, path: &str) -> Vec<ValueSource> {
        vec![ValueSource {
            source_id: self.source.id.clone(),
            pointer: format!("{}{path}", self.pointer),
        }]
    }

    pub fn derived(&mut self, name: &str, value: Value, pointer: &str, unit: Option<&str>) {
        self.record.fields.insert(
            name.into(),
            CatalogValue {
                value,
                unit: unit.map(str::to_owned),
                status: ValueStatus::Derived,
                sources: self.origin(pointer),
            },
        );
    }

    pub fn value(&mut self, path: &str, shape: Shape, unit: Option<&str>) -> Option<CatalogValue> {
        let raw = self.entry.pointer(path)?;
        let accepted = match shape {
            Shape::Text => raw.is_string(),
            Shape::Number => raw.is_number(),
            Shape::Bool => raw.is_boolean(),
            Shape::Strings => raw
                .as_array()
                .is_some_and(|a| a.iter().all(Value::is_string)),
            Shape::Ids => raw
                .as_array()
                .is_some_and(|a| a.iter().all(|v| positive_id(v).is_some())),
            Shape::Numbers => raw
                .as_array()
                .is_some_and(|a| a.iter().all(Value::is_number)),
            Shape::BoolMap => raw.as_object().is_some_and(|a| {
                a.iter()
                    .all(|(k, v)| k.parse::<u32>().is_ok() && v.is_boolean())
            }),
        };
        let status = if raw.is_null() {
            ValueStatus::Missing
        } else if accepted {
            ValueStatus::Verified
        } else {
            ValueStatus::Unsupported
        };
        let value = if accepted && matches!(shape, Shape::Ids) {
            Value::Array(
                raw.as_array()?
                    .iter()
                    .filter_map(positive_id)
                    .map(Value::String)
                    .collect(),
            )
        } else {
            raw.clone()
        };
        if status == ValueStatus::Unsupported {
            self.issue(format!("invalid_field:{}{path}", self.pointer));
        } else {
            self.consume(path);
        }
        Some(CatalogValue {
            value,
            unit: unit.map(str::to_owned),
            status,
            sources: self.origin(path),
        })
    }

    pub fn field(&mut self, name: &str, path: &str, shape: Shape, unit: Option<&str>) {
        if let Some(value) = self.value(path, shape, unit) {
            self.record.fields.insert(name.into(), value);
        }
    }

    pub fn text(&mut self, name: &str, path: &str) {
        if let Some(mut value) = self.value(path, Shape::Text, None) {
            if let Some(raw) = value.value.as_str() {
                value.value = Value::String(super::plain_text(raw));
                value.status = ValueStatus::Descriptive;
            }
            self.record.fields.insert(name.into(), value);
        }
    }

    /// Segments typés de l'infobulle, à côté du texte brut `tooltip` qui reste inchangé.
    /// Le type vient des balises de dégâts de la source : transformation déterministe, donc `derived`.
    pub fn tooltip_segments(&mut self, name: &str, path: &str) {
        let Some(raw) = self.entry.pointer(path).and_then(Value::as_str) else {
            return;
        };
        let segments = tooltip_segments(raw);
        self.derived(name, json!(segments), path, None);
    }

    pub fn name(&mut self, path: &str) {
        self.text("name", path);
        self.record.name = self
            .record
            .fields
            .get("name")
            .and_then(|v| v.value.as_str())
            .unwrap_or_default()
            .into();
        if !self.entry.pointer(path).is_some_and(Value::is_string) {
            self.issue("missing_name");
        }
    }

    pub fn description(&mut self, path: &str) {
        self.text("description", path);
        self.record.description = self
            .record
            .fields
            .get("description")
            .and_then(|v| v.value.as_str())
            .map(str::to_owned);
        if self
            .record
            .description
            .as_ref()
            .is_some_and(|v| v.contains("{{") || v.contains('@'))
        {
            self.issue("unresolved_description");
        }
    }

    pub fn icon(&mut self, path: &str, version: &str, group: &str) {
        let Some(raw) = self.entry.pointer(path).and_then(Value::as_str) else {
            return;
        };
        let valid = !raw.is_empty()
            && !raw.contains("..")
            && raw.bytes().all(|c| {
                c.is_ascii_alphanumeric() || b"_-.".contains(&c) || (group == "rune" && c == b'/')
            });
        if !valid || (group == "rune" && !raw.starts_with("perk-images/")) {
            self.issue("invalid_icon");
            return;
        }
        let url = if group == "rune" {
            format!("https://ddragon.leagueoflegends.com/cdn/img/{raw}")
        } else {
            let namespace = if self.record.namespace == "classic" {
                "mode/classic/"
            } else {
                ""
            };
            format!(
                "https://ddragon.leagueoflegends.com/cdn/{version}/img/{namespace}{group}/{raw}"
            )
        };
        self.consume(path);
        self.derived("icon", json!(url), path, None);
        self.record.icon = Some(url);
    }

    pub fn stats(&mut self, mapping: &[(&str, &str, &str)]) {
        for (raw, name, unit) in mapping {
            let unit = (!unit.is_empty()).then_some(*unit);
            if let Some(value) = self.value(&format!("/stats/{raw}"), Shape::Number, unit) {
                self.record.stats.insert((*name).into(), value);
            }
        }
    }

    pub fn issue(&mut self, issue: impl Into<String>) {
        self.record.coverage.issues.push(issue.into());
    }

    pub fn finish(mut self) -> CatalogRecord {
        self.record.coverage.source_fields = self.paths.len() as u32;
        self.record.coverage.normalized_fields = self.used.len() as u32;
        self.record.coverage.unmapped_fields = self
            .paths
            .difference(&self.used)
            .map(|pointer| format!("{}:{pointer}", self.source.id))
            .collect();
        self.record.coverage.issues.sort();
        self.record.coverage.issues.dedup();
        self.record
    }
}

pub(super) fn positive_id(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.as_u64()?.to_string(),
        _ => return None,
    };
    text.parse::<u32>()
        .ok()
        .filter(|n| *n > 0)
        .map(|n| n.to_string())
}

/// Fragment lexical d'un texte balisé, une fois les entités décodées.
enum Piece {
    Text(String),
    /// Séparation sans balise (`>` isolé, `<` non fermé, balise auto-fermante, bloc actif retiré).
    Gap,
    Open(String),
    Close(String),
}

fn decode_entities(input: &str) -> String {
    let mut decoded = String::new();
    let mut rest = input;
    while !rest.is_empty() {
        if rest.starts_with('&') {
            if let Some(end) = rest.find(';').filter(|end| *end <= 16) {
                let entity = &rest[1..end];
                let character = match entity {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" | "#39" => Some('\''),
                    "nbsp" => Some(' '),
                    _ => entity
                        .strip_prefix("#x")
                        .or_else(|| entity.strip_prefix("#X"))
                        .and_then(|s| u32::from_str_radix(s, 16).ok())
                        .or_else(|| entity.strip_prefix('#').and_then(|s| s.parse().ok()))
                        .and_then(char::from_u32),
                };
                if let Some(character) = character {
                    decoded.push(character);
                    rest = &rest[end + 1..];
                    continue;
                }
            }
        }
        if let Some(character) = rest.chars().next() {
            decoded.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    decoded
}

/// Découpe le texte décodé ; le contenu des blocs `script` et `style` est retiré.
fn markup_pieces(decoded: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut rest = decoded;
    while !rest.is_empty() {
        if rest.starts_with('<') {
            if !text.is_empty() {
                pieces.push(Piece::Text(std::mem::take(&mut text)));
            }
            let Some(end) = rest.find('>') else {
                pieces.push(Piece::Gap);
                rest = &rest[1..];
                continue;
            };
            let tag = rest[1..end].trim().to_ascii_lowercase();
            let name = tag
                .split_ascii_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_owned();
            rest = &rest[end + 1..];
            if matches!(name.as_str(), "script" | "style") {
                let close = format!("</{name}");
                if let Some(index) = rest.to_ascii_lowercase().find(&close) {
                    rest = &rest[index..];
                    if let Some(end) = rest.find('>') {
                        rest = &rest[end + 1..];
                    } else {
                        rest = "";
                    }
                } else {
                    rest = "";
                }
                pieces.push(Piece::Gap);
            } else if let Some(closed) = name.strip_prefix('/') {
                pieces.push(Piece::Close(closed.to_owned()));
            } else if tag.ends_with('/') || name.is_empty() {
                pieces.push(Piece::Gap);
            } else {
                pieces.push(Piece::Open(name));
            }
            continue;
        }
        if let Some(character) = rest.chars().next() {
            if character == '>' {
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(Piece::Gap);
            } else {
                text.push(character);
            }
            rest = &rest[character.len_utf8()..];
        }
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    pieces
}

/// Retire le balisage et les blocs actifs ; le résultat est exclusivement du texte.
pub(super) fn plain_text(input: &str) -> String {
    let mut output = String::new();
    for piece in markup_pieces(&decode_entities(input)) {
        match piece {
            Piece::Text(text) => output.push_str(&text),
            _ => output.push(' '),
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn damage_type_of(tag: &str) -> Option<DamageType> {
    match tag {
        "physicaldamage" => Some(DamageType::Physical),
        "magicdamage" => Some(DamageType::Magic),
        "truedamage" => Some(DamageType::True),
        _ => None,
    }
}

/// Découpe une infobulle en fragments de texte typés par les balises de dégâts de Data Dragon.
/// Le texte concaténé est exactement celui de `plain_text` ; seules les trois balises de dégâts
/// donnent un type, les autres balises (`status`, `speed`, `scaleAP`…) n'en donnent pas et héritent
/// de celui qui les entoure. Le type le plus interne l'emporte ; une fermeture orpheline est ignorée.
pub(super) fn tooltip_segments(input: &str) -> Vec<TooltipSegment> {
    let mut segments: Vec<TooltipSegment> = Vec::new();
    let mut open: Vec<(String, Option<DamageType>)> = Vec::new();
    // Espace en attente : écrit seulement devant le prochain caractère visible, avec le type en
    // vigueur au dernier espace rencontré ; les espaces de tête et de queue disparaissent.
    let mut pending: Option<Option<DamageType>> = None;
    let current = |open: &[(String, Option<DamageType>)]| open.iter().rev().find_map(|(_, t)| *t);
    for piece in markup_pieces(&decode_entities(input)) {
        match piece {
            Piece::Gap => pending = Some(current(&open)),
            Piece::Open(name) => {
                pending = Some(current(&open));
                let kind = damage_type_of(&name);
                open.push((name, kind));
            }
            Piece::Close(name) => {
                if let Some(index) = open.iter().rposition(|(open_name, _)| *open_name == name) {
                    open.truncate(index);
                }
                pending = Some(current(&open));
            }
            Piece::Text(text) => {
                let kind = current(&open);
                for character in text.chars() {
                    if character.is_whitespace() {
                        pending = Some(kind);
                        continue;
                    }
                    if let Some(space_kind) = pending.take() {
                        if !segments.is_empty() {
                            push_segment(&mut segments, " ", space_kind);
                        }
                    }
                    push_segment(&mut segments, character.encode_utf8(&mut [0; 4]), kind);
                }
            }
        }
    }
    segments
}

fn push_segment(segments: &mut Vec<TooltipSegment>, text: &str, damage_type: Option<DamageType>) {
    match segments.last_mut() {
        Some(last) if last.damage_type == damage_type => last.text.push_str(text),
        _ => segments.push(TooltipSegment {
            text: text.into(),
            damage_type,
        }),
    }
}

#[cfg(test)]
#[path = "project_value_tests.rs"]
mod tests;
