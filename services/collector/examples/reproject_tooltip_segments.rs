//! Reprise ciblée du catalogue desktop depuis les sources Data Dragon originales (#107).
use clap::Parser;
use olc_catalog_cache::{
    digest, snapshot_id, validate_catalog, validate_manifest, Cache, FileEntry, MAX_JSON,
    MAX_MANIFEST, MAX_TOTAL,
};
use olc_collector::catalog::{
    make_source, project_sources, valid_version, CatalogRecord, CatalogSource, SourceMeta,
    TooltipSegment, ValueStatus, NORMALIZER_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const PREFIX: &str = "/game-data/catalog/";
const LOCALES: [&str; 2] = ["fr_FR", "en_US"];
const MAX_SOURCES: usize = 1000;

fn source_url(meta: &SourceMeta, version: &str) -> Result<String> {
    let locale = meta.locale.as_deref().ok_or("locale de source absente")?;
    let resource = meta
        .key
        .strip_prefix(&format!("{locale}/"))
        .ok_or("clé de source invalide")?;
    let champion = resource
        .strip_prefix("champion/")
        .and_then(|name| name.strip_suffix(".json"));
    if !valid_version(version)
        || meta.provider != "ddragon"
        || meta.version != version
        || !LOCALES.contains(&locale)
        || !olc_catalog_cache::valid_id(&meta.id)
        || !(resource == "summoner.json"
            || champion.is_some_and(|key| {
                !key.is_empty() && key.len() <= 64 && key.bytes().all(|c| c.is_ascii_alphanumeric())
            }))
    {
        return Err("contexte de source publique invalide".into());
    }
    let url = format!("https://ddragon.leagueoflegends.com/cdn/{version}/data/{locale}/{resource}");
    if meta.url != url {
        return Err("URL de source divergente".into());
    }
    Ok(url)
}

fn checked_source(meta: &SourceMeta, version: &str, data: Value) -> Result<CatalogSource> {
    source_url(meta, version)?;
    let source = make_source(
        &meta.provider,
        &meta.key,
        &meta.version,
        meta.locale.as_deref(),
        &meta.url,
        &meta.observed_at,
        data,
    );
    if source.id != meta.id {
        return Err(format!("empreinte de source divergente : {}", meta.key).into());
    }
    Ok(source)
}

fn meta(source: &CatalogSource) -> SourceMeta {
    SourceMeta {
        id: source.id.clone(),
        provider: source.provider.clone(),
        key: source.key.clone(),
        version: source.version.clone(),
        locale: source.locale.clone(),
        url: source.url.clone(),
        observed_at: source.observed_at.clone(),
    }
}

fn identity(record: &CatalogRecord) -> (&str, &str, &str, &str) {
    (&record.kind, &record.namespace, &record.locale, &record.id)
}

fn reproject_records(
    version: &str,
    records: &[CatalogRecord],
    sources: &[CatalogSource],
) -> Result<Vec<CatalogRecord>> {
    if sources.is_empty() || sources.len() > MAX_SOURCES {
        return Err("sources absentes ou trop nombreuses".into());
    }
    let mut ids = BTreeSet::new();
    for source in sources {
        checked_source(&meta(source), version, source.data.clone())?;
        if !ids.insert(&source.id) {
            return Err("source dupliquée".into());
        }
    }
    let projection = project_sources(version, sources.to_vec(), false, vec![])?;
    let projected: BTreeMap<_, _> = projection
        .records
        .iter()
        .map(|record| (identity(record), record))
        .collect();
    let mut result = records.to_vec();
    for record in &mut result {
        if !matches!(record.kind.as_str(), "ability" | "summoner_spell") {
            continue;
        }
        let Some(tooltip) = record.fields.get("tooltip") else {
            continue;
        };
        if record.namespace != "standard"
            || !LOCALES.contains(&record.locale.as_str())
            || !tooltip.value.is_string()
            || tooltip.sources.is_empty()
        {
            return Err("tooltip d’origine invalide".into());
        }
        let fresh = projected
            .get(&identity(record))
            .ok_or("record absent de la reprojection")?;
        let fresh_tooltip = fresh
            .fields
            .get("tooltip")
            .ok_or("tooltip absent de la source")?;
        if fresh_tooltip != tooltip {
            return Err(format!(
                "tooltip ou provenance divergente : {} {}",
                record.locale, record.id
            )
            .into());
        }
        let field = fresh
            .fields
            .get("tooltip_segments")
            .ok_or("segments absents du normaliseur")?;
        let segments: Vec<TooltipSegment> = serde_json::from_value(field.value.clone())?;
        if field.status != ValueStatus::Derived
            || field.unit.is_some()
            || field.sources != tooltip.sources
            || segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<String>()
                != tooltip.value.as_str().ok_or("tooltip invalide")?
        {
            return Err("invariant des segments non respecté".into());
        }
        record
            .fields
            .insert("tooltip_segments".into(), field.clone());
    }
    Ok(result)
}

#[derive(Parser)]
#[command(
    about = "Reprojette uniquement tooltip_segments depuis les raw publics exacts, sans base ni téléchargement"
)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    /// JSON bruts nommés <source_id>.json ; aucune valeur normalisée n’est une source brute.
    #[arg(long)]
    raw_dir: PathBuf,
    /// Export schéma 2 : dossier absent ou vide, dont le parent existe.
    #[arg(long)]
    output: PathBuf,
    /// Cache immuable d’origine, ouvert en lecture seule.
    #[arg(long)]
    snapshot_input: PathBuf,
    /// Nouveau cache : dossier absent ou vide, distinct de l’origine.
    #[arg(long)]
    snapshot_output: PathBuf,
    #[arg(long, default_value_t = 726)]
    expected_tooltips: usize,
}

#[derive(Clone, Deserialize, Serialize)]
struct Document {
    version: String,
    records: Vec<CatalogRecord>,
}
type Files = BTreeMap<String, Vec<u8>>;

fn read_file(root: &Path, relative: &str, limit: u64) -> Result<Vec<u8>> {
    if !olc_catalog_cache::valid_path(relative) {
        return Err("chemin de fichier invalide".into());
    }
    let mut path = root.to_path_buf();
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err("lien symbolique interdit".into());
    }
    for part in relative.split('/') {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err("lien symbolique interdit".into());
        }
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("fichier absent ou trop grand".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("fichier trop grand".into());
    }
    Ok(bytes)
}

fn read_metadata(input: &Path, value: &Value, files: &mut Files) -> Result<String> {
    let relative = value["path"]
        .as_str()
        .or_else(|| value["local_path"].as_str())
        .and_then(|path| path.strip_prefix(PREFIX))
        .ok_or("préfixe de fichier invalide")?;
    let bytes = read_file(
        input,
        relative,
        if relative.ends_with(".json") {
            MAX_JSON
        } else {
            2 * 1024 * 1024
        },
    )?;
    if value["bytes"].as_u64() != Some(bytes.len() as u64)
        || value["sha256"].as_str() != Some(digest(&bytes).as_str())
    {
        return Err("empreinte d’export invalide".into());
    }
    let used: u64 = files.values().map(|bytes| bytes.len() as u64).sum();
    if files.len() >= 10000 || used.saturating_add(bytes.len() as u64) > MAX_TOTAL {
        return Err("export trop grand".into());
    }
    if let Some(old) = files.insert(relative.into(), bytes.clone()) {
        if old != bytes {
            return Err("fichier dupliqué divergent".into());
        }
    }
    Ok(relative.into())
}

fn file_metadata(path: &str, bytes: &[u8]) -> Value {
    json!({"path":format!("{PREFIX}{path}"),"bytes":bytes.len(),"sha256":digest(bytes)})
}

fn empty_output(output: &Path, input: &Path) -> Result<PathBuf> {
    let name = output.file_name().ok_or("nom de sortie absent")?;
    let destination = output
        .parent()
        .ok_or("parent de sortie absent")?
        .canonicalize()?
        .join(name);
    if output_key(&destination)?.starts_with(&output_key(&input.canonicalize()?)?) {
        return Err("sortie dans l’origine interdite".into());
    }
    match fs::symlink_metadata(output) {
        Ok(metadata)
            if metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && fs::read_dir(output)?.next().is_none() =>
        {
            Ok(destination)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(destination),
        _ => Err("sortie déjà présente ou non vide".into()),
    }
}

fn write_export(output: &Path, files: &Files) -> Result<()> {
    fs::create_dir_all(output)?;
    for (relative, bytes) in files {
        let path = output.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    Ok(())
}

fn checked_outputs(
    output: &Path,
    snapshot_output: &Path,
    inputs: &[&Path],
) -> Result<(PathBuf, PathBuf)> {
    let (first, remaining) = inputs.split_first().ok_or("origines absentes")?;
    let output = empty_output(output, first)?;
    let snapshot_output = empty_output(snapshot_output, first)?;
    for input in remaining {
        empty_output(&output, input)?;
        empty_output(&snapshot_output, input)?;
    }
    // Comparer les destinations résolues, avant toute création : les chemins
    // relatifs et symlinks d’ancêtres peuvent désigner le même dossier.
    // Refus conservateur même sur Linux : les feuilles encore absentes ne
    // peuvent pas être canonicalisées, et macOS/Windows ignorent souvent la casse.
    let export_key = output_key(&output)?;
    let snapshot_key = output_key(&snapshot_output)?;
    if export_key.starts_with(&snapshot_key) || snapshot_key.starts_with(&export_key) {
        return Err("sorties identiques ou imbriquées".into());
    }
    Ok((output, snapshot_output))
}

fn output_key(path: &Path) -> Result<Vec<String>> {
    path.components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .map(str::to_uppercase)
                .ok_or_else(|| "chemin de sortie non Unicode".into())
        })
        .collect()
}

fn derive_snapshot(
    input: &Path,
    output: &Path,
    version: &str,
    before: &Files,
    after: &Files,
) -> Result<(Cache, olc_catalog_cache::Manifest)> {
    let source = Cache::reader(input)?;
    let original = source.release(version)?;
    validate_catalog(&source, &original)?;
    for (path, bytes) in before {
        let entry = original
            .files
            .get(&format!("catalog/{path}"))
            .ok_or("export absent du snapshot d’origine")?;
        if entry.sha256 != digest(bytes) || entry.bytes != bytes.len() as u64 {
            return Err("export et snapshot d’origine différents".into());
        }
    }
    if original
        .files
        .keys()
        .filter(|path| path.starts_with("catalog/"))
        .count()
        != before.len()
    {
        return Err("fichiers d’origine non repris".into());
    }
    let mut manifest = original.clone();
    manifest.normalizer_version = NORMALIZER_VERSION;
    for (path, bytes) in after {
        let key = format!("catalog/{path}");
        let media_type = original
            .files
            .get(&key)
            .ok_or("nouveau fichier hors périmètre")?
            .media_type
            .clone();
        manifest.files.insert(
            key,
            FileEntry {
                bytes: bytes.len() as u64,
                sha256: digest(bytes),
                media_type,
            },
        );
    }
    manifest.snapshot_id = snapshot_id(&manifest)?;
    validate_manifest(&manifest)?;
    let mut destination = Cache::open(output)?;
    for (path, entry) in &manifest.files {
        let bytes = match path
            .strip_prefix("catalog/")
            .and_then(|relative| after.get(relative))
        {
            Some(bytes) => bytes.clone(),
            None => source.read(&original.snapshot_id, path)?,
        };
        destination.put(entry, &bytes)?;
    }
    destination.save(&manifest)?;
    validate_catalog(&destination, &manifest)?;
    if source.release(version)?.snapshot_id != original.snapshot_id {
        return Err("pointeur d’origine modifié pendant la reprise".into());
    }
    Ok((destination, manifest))
}

fn write_outputs(
    input: &Path,
    snapshot_output: &Path,
    export_output: &Path,
    version: &str,
    before: &Files,
    after: &Files,
) -> Result<String> {
    let (mut destination, manifest) =
        derive_snapshot(input, snapshot_output, version, before, after)?;
    write_export(export_output, after)?;
    // Le cache garde son verrou ; un échec de l’export laisse des fichiers
    // préparés pour diagnostic, mais aucun nouveau pointeur publié.
    destination.publish(&manifest)?;
    Ok(manifest.snapshot_id)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let (output, snapshot_output) = checked_outputs(
        &args.output,
        &args.snapshot_output,
        &[&args.input, &args.snapshot_input, &args.raw_dir],
    )?;
    let manifest_bytes = read_file(&args.input, "manifest.json", MAX_MANIFEST as u64)?;
    let mut manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    if manifest["schema_version"] != 2 {
        return Err("schéma export invalide".into());
    }
    let version = manifest["version"]
        .as_str()
        .filter(|version| valid_version(version))
        .ok_or("version invalide")?
        .to_owned();
    let mut files = BTreeMap::from([("manifest.json".into(), manifest_bytes)]);
    let source_path = read_metadata(&args.input, &manifest["sources"], &mut files)?;
    let mut provenance: Value = serde_json::from_slice(&files[&source_path])?;
    if provenance["version"] != version
        || provenance["normalizer_version"] != manifest["normalizer_version"]
    {
        return Err("provenance d’export incohérente".into());
    }
    for icon in provenance["icons"].as_array().ok_or("icônes absentes")? {
        read_metadata(&args.input, icon, &mut files)?;
    }
    let mut document_paths = Vec::new();
    for locale in LOCALES {
        document_paths.push(read_metadata(
            &args.input,
            &manifest["locales"][locale],
            &mut files,
        )?);
    }
    for locales in manifest["champions"]
        .as_object()
        .ok_or("détails absents")?
        .values()
    {
        for locale in LOCALES {
            document_paths.push(read_metadata(&args.input, &locales[locale], &mut files)?);
        }
    }
    let mut documents = BTreeMap::new();
    let mut records = Vec::new();
    for path in &document_paths {
        let value: Value = serde_json::from_slice(&files[path])?;
        let document: Document = serde_json::from_value(value.clone())?;
        if document.version != version || serde_json::to_value(&document)? != value {
            return Err("document non préservable exactement".into());
        }
        records.extend(document.records.clone());
        documents.insert(path.clone(), document);
    }
    let required: BTreeSet<_> = records
        .iter()
        .filter(|record| matches!(record.kind.as_str(), "ability" | "summoner_spell"))
        .filter_map(|record| record.fields.get("tooltip"))
        .flat_map(|tooltip| {
            tooltip
                .sources
                .iter()
                .map(|origin| origin.source_id.clone())
        })
        .collect();
    if required.is_empty() || required.len() > MAX_SOURCES {
        return Err("nombre de sources invalide".into());
    }
    let metadata: Vec<SourceMeta> = serde_json::from_value(provenance["sources"].clone())?;
    let mut sources = Vec::new();
    for expected in metadata
        .iter()
        .filter(|source| required.contains(&source.id))
    {
        source_url(expected, &version)?;
        let bytes = read_file(&args.raw_dir, &format!("{}.json", expected.id), MAX_JSON)?;
        sources.push(checked_source(
            expected,
            &version,
            serde_json::from_slice(&bytes)?,
        )?);
    }
    if sources.len() != required.len() {
        return Err("provenance raw incomplète ou dupliquée".into());
    }
    let projected = reproject_records(&version, &records, &sources)?;
    let by_id: BTreeMap<_, _> = projected
        .iter()
        .map(|record| (identity(record), record))
        .collect();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut segment_bytes = 0usize;
    let before = files.clone();
    for (path, document) in &mut documents {
        for record in &mut document.records {
            let fresh = by_id.get(&identity(record)).ok_or("record perdu")?;
            *record = (**fresh).clone();
            if matches!(record.kind.as_str(), "ability" | "summoner_spell")
                && record.fields.contains_key("tooltip_segments")
            {
                *counts.entry(record.locale.clone()).or_default() += 1;
            }
        }
        let bytes = serde_json::to_vec(document)?;
        let mut stripped = document.clone();
        for record in &mut stripped.records {
            record.fields.remove("tooltip_segments");
        }
        segment_bytes += bytes
            .len()
            .saturating_sub(serde_json::to_vec(&stripped)?.len());
        files.insert(path.clone(), bytes);
    }
    if counts.len() != 2
        || LOCALES
            .iter()
            .any(|locale| counts.get(*locale) != Some(&args.expected_tooltips))
    {
        return Err("nombre de tooltips inattendu".into());
    }
    let scope = json!({"fields":["tooltip_segments"],"base_normalizer_version":manifest["normalizer_version"],"other_records_preserved":true});
    provenance["normalizer_version"] = json!(NORMALIZER_VERSION);
    provenance["reprojection"] = scope.clone();
    files.insert(source_path.clone(), serde_json::to_vec(&provenance)?);
    manifest["normalizer_version"] = json!(NORMALIZER_VERSION);
    manifest["reprojection"] = scope;
    manifest["sources"] = file_metadata(&source_path, &files[&source_path]);
    for locale in LOCALES {
        let path = manifest["locales"][locale]["path"]
            .as_str()
            .and_then(|path| path.strip_prefix(PREFIX))
            .ok_or("racine invalide")?
            .to_owned();
        manifest["locales"][locale]["bytes"] = json!(files[&path].len());
        manifest["locales"][locale]["sha256"] = json!(digest(&files[&path]));
    }
    for locales in manifest["champions"]
        .as_object_mut()
        .ok_or("détails invalides")?
        .values_mut()
    {
        for locale in LOCALES {
            let path = locales[locale]["path"]
                .as_str()
                .and_then(|path| path.strip_prefix(PREFIX))
                .ok_or("détail invalide")?
                .to_owned();
            locales[locale]["bytes"] = json!(files[&path].len());
            locales[locale]["sha256"] = json!(digest(&files[&path]));
        }
    }
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    let snapshot = write_outputs(
        &args.snapshot_input,
        &snapshot_output,
        &output,
        &version,
        &before,
        &files,
    )?;
    println!(
        "{}",
        json!({"version":version,"sources":sources.len(),"tooltips":counts,"segment_field_bytes":segment_bytes,"export_bytes_before":before.values().map(Vec::len).sum::<usize>(),"export_bytes_after":files.values().map(Vec::len).sum::<usize>(),"snapshot_id":snapshot})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use olc_collector::catalog::{make_source, project_sources, CatalogValue, ValueStatus};
    use serde_json::json;

    fn fixture() -> (Vec<CatalogRecord>, CatalogSource) {
        let source = make_source(
            "ddragon",
            "fr_FR/summoner.json",
            "16.19.1",
            Some("fr_FR"),
            "https://ddragon.leagueoflegends.com/cdn/16.19.1/data/fr_FR/summoner.json",
            "original",
            json!({"type":"summoner","version":"16.19.1","data":{"SummonerDot":{"id":"SummonerDot","key":"14","name":"Embrasement","description":"Description","tooltip":"Inflige <trueDamage>{{ amount }}</trueDamage>.","image":{"full":"SummonerDot.png"},"cooldown":[180],"modes":["CLASSIC"]}}}),
        );
        let mut records = project_sources("16.19.1", vec![source.clone()], false, vec![])
            .unwrap()
            .records;
        for record in &mut records {
            record.fields.remove("tooltip_segments");
            record.fields.insert(
                "preserved_marker".into(),
                CatalogValue {
                    value: json!(42),
                    status: ValueStatus::Verified,
                    unit: None,
                    sources: vec![],
                },
            );
        }
        (records, source)
    }

    #[test]
    fn copie_uniquement_les_segments_et_garde_les_autres_valeurs() {
        let (before, source) = fixture();
        let mut after = reproject_records("16.19.1", &before, &[source]).unwrap();
        assert!(after[0].fields.contains_key("tooltip_segments"));
        assert_eq!(
            after[0].fields["tooltip_segments"].status,
            ValueStatus::Derived
        );
        assert!(after[0].fields["tooltip_segments"]
            .value
            .as_array()
            .unwrap()
            .iter()
            .any(|segment| segment["damage_type"] == "true"));
        after[0].fields.remove("tooltip_segments");
        assert_eq!(after, before);
    }

    #[test]
    fn refuse_un_id_de_source_modifie_sans_recycler_la_provenance() {
        let (before, mut source) = fixture();
        source.data["data"]["SummonerDot"]["name"] = json!("Modifié");
        assert!(reproject_records("16.19.1", &before, &[source]).is_err());
    }

    #[test]
    fn refuse_un_tooltip_divergent_et_une_source_absente() {
        let (mut before, source) = fixture();
        assert!(reproject_records("16.19.1", &before, &[]).is_err());
        before[0].fields.get_mut("tooltip").unwrap().value = json!("autre texte");
        assert!(reproject_records("16.19.1", &before, &[source]).is_err());
    }

    #[test]
    fn une_autre_famille_et_un_passif_sont_conserves_sans_reprojection() {
        let (mut before, source) = fixture();
        let mut item = before[0].clone();
        item.kind = "item".into();
        item.id = "3078".into();
        let mut passive = before[0].clone();
        passive.kind = "ability".into();
        passive.id = "103:passive".into();
        passive.fields.remove("tooltip");
        before.extend([item, passive]);
        let after = reproject_records("16.19.1", &before, &[source]).unwrap();
        assert_eq!(after[1..], before[1..]);
    }

    #[test]
    fn refuse_les_contextes_publics_non_epingles() {
        let (_, source) = fixture();
        let expected = meta(&source);
        let mut wrong = expected.clone();
        wrong.url = wrong.url.replace("16.19.1", "latest");
        assert!(source_url(&wrong, "16.19.1").is_err());
        wrong = expected.clone();
        wrong.url = "https://example.invalid/summoner.json".into();
        assert!(source_url(&wrong, "16.19.1").is_err());
        wrong = expected;
        wrong.key = "fr_FR/../summoner.json".into();
        assert!(source_url(&wrong, "16.19.1").is_err());
    }

    #[test]
    fn borne_la_lecture_et_refuse_les_chemins_sortant_du_dossier() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("data.json"), b"{} ").unwrap();
        assert_eq!(read_file(directory.path(), "data.json", 3).unwrap(), b"{} ");
        assert!(read_file(directory.path(), "data.json", 2).is_err());
        assert!(read_file(directory.path(), "../data.json", 10).is_err());
    }

    #[test]
    fn ne_touche_pas_aux_sorties_occupees_ou_a_l_origine() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        fs::create_dir(&input).unwrap();
        fs::write(input.join("guard.json"), b"original").unwrap();
        assert!(empty_output(&input.join("new"), &input).is_err());
        assert!(empty_output(&input, &input).is_err());
        let output = directory.path().join("new");
        assert!(empty_output(&output, &input).is_ok());
        fs::create_dir(&output).unwrap();
        fs::write(output.join("guard.json"), b"keep").unwrap();
        assert!(empty_output(&output, &input).is_err());
        assert_eq!(fs::read(input.join("guard.json")).unwrap(), b"original");
        assert_eq!(fs::read(output.join("guard.json")).unwrap(), b"keep");
    }

    #[test]
    fn rejects_equivalent_output_paths_before_writing() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        let helper = directory.path().join("helper");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&helper).unwrap();
        let output = directory.path().join("new");
        let alias = helper.join("../new");
        assert!(checked_outputs(&output, &alias, &[&input]).is_err());
        assert!(!output.exists());
    }

    #[test]
    fn rejects_case_variant_outputs_before_writing_on_every_os() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        fs::create_dir(&input).unwrap();
        let output = directory.path().join("ReviewOutput");
        let alias = directory.path().join("reviewoutput");
        assert!(checked_outputs(&output, &alias, &[&input]).is_err());
        assert!(!output.exists());
        assert!(!alias.exists());
    }

    #[test]
    fn rejects_an_output_inside_a_case_variant_of_the_origin() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("Origin");
        fs::create_dir(&input).unwrap();
        fs::write(input.join("guard.json"), b"original").unwrap();
        let output = directory.path().join("origin/new");
        assert!(empty_output(&output, &input).is_err());
        assert!(!input.join("new").exists());
        assert_eq!(fs::read(input.join("guard.json")).unwrap(), b"original");
    }

    #[test]
    fn rejects_nested_outputs_in_both_directions() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        let output = directory.path().join("new");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&output).unwrap();
        let child = output.join("cache");
        assert!(checked_outputs(&output, &child, &[&input]).is_err());
        assert!(checked_outputs(&child, &output, &[&input]).is_err());
        assert_eq!(fs::read_dir(output).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_outputs_aliased_by_an_ancestor_symlink() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        let parent = directory.path().join("parent");
        let alias = directory.path().join("alias");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&parent).unwrap();
        std::os::unix::fs::symlink(&parent, &alias).unwrap();
        assert!(checked_outputs(&parent.join("new"), &alias.join("new"), &[&input]).is_err());
        assert!(!parent.join("new").exists());
    }

    fn snapshot_fixture(root: &Path) -> (olc_catalog_cache::Manifest, Files) {
        let version = "16.19.1";
        let mut files = BTreeMap::from([
            ("champions.json".to_owned(), serde_json::to_vec(&json!({"103":{"key":"Ahri","fr":"Ahri","en":"Ahri"}})).unwrap()),
            ("champion-directory.json".to_owned(), serde_json::to_vec(&json!({"version":version,"champions":[{"id":103,"key":"Ahri","names":{"fr":"Ahri","en":"Ahri"},"titles":{"fr":"Renarde","en":"Fox"},"categories":["Mage"],"image":"champions/103.png"}]})).unwrap()),
            ("champions/103.png".to_owned(), b"image".to_vec()),
        ]);
        let mut export = Files::new();
        for locale in LOCALES {
            let record = |kind: &str, id: &str| json!({"kind":kind,"id":id,"locale":locale,"namespace":"standard","name":"Ahri","description":null,"icon":null,"fields":{},"stats":{},"effects":[],"coverage":{}});
            export.insert(format!("{locale}.json"), serde_json::to_vec(&json!({"version":version,"records":[record("item","1"),record("rune","2"),record("rune_shard","3"),record("summoner_spell","4")]})).unwrap());
            let mut abilities = vec![record("champion", "103")];
            abilities.extend(
                ["Q", "W", "E", "R", "passive"]
                    .map(|slot| record("ability", &format!("103:{slot}"))),
            );
            export.insert(
                format!("champions/103/{locale}.json"),
                serde_json::to_vec(&json!({"version":version,"records":abilities})).unwrap(),
            );
        }
        files.extend(
            export
                .iter()
                .map(|(path, bytes)| (format!("catalog/{path}"), bytes.clone())),
        );
        let mut manifest = olc_catalog_cache::Manifest {
            schema_version: 1,
            version: version.into(),
            normalizer_version: 1,
            snapshot_id: String::new(),
            files: BTreeMap::new(),
        };
        let mut cache = Cache::open(root).unwrap();
        for (path, bytes) in files {
            let entry = FileEntry {
                bytes: bytes.len() as u64,
                sha256: digest(&bytes),
                media_type: if path.ends_with(".png") {
                    "image/png"
                } else {
                    "application/json"
                }
                .into(),
            };
            cache.put(&entry, &bytes).unwrap();
            manifest.files.insert(path, entry);
        }
        manifest.snapshot_id = snapshot_id(&manifest).unwrap();
        cache.save(&manifest).unwrap();
        validate_catalog(&cache, &manifest).unwrap();
        cache.publish(&manifest).unwrap();
        (manifest, export)
    }

    #[test]
    fn failed_export_never_publishes_a_new_head_or_changes_the_origin() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        let (original, before) = snapshot_fixture(&input);
        let export = directory.path().join("export");
        let snapshot = directory.path().join("snapshot");
        checked_outputs(&export, &snapshot, &[&input]).unwrap();
        // Une indisponibilité I/O après le précontrôle ne doit pas publier le cache préparé.
        fs::write(&export, b"blocked").unwrap();
        assert!(write_outputs(
            &input,
            &snapshot,
            &export,
            &original.version,
            &before,
            &before
        )
        .is_err());
        assert!(!snapshot.join("heads").join(&original.version).exists());
        let source = Cache::reader(&input).unwrap();
        assert_eq!(
            source.release(&original.version).unwrap().snapshot_id,
            original.snapshot_id
        );
        validate_catalog(&source, &original).unwrap();
        assert_eq!(fs::read(export).unwrap(), b"blocked");
    }

    #[test]
    fn successful_export_publishes_the_validated_snapshot_last() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("origin");
        let (original, before) = snapshot_fixture(&input);
        let export = directory.path().join("export");
        let snapshot = directory.path().join("snapshot");
        let id = write_outputs(
            &input,
            &snapshot,
            &export,
            &original.version,
            &before,
            &before,
        )
        .unwrap();
        let target = Cache::reader(&snapshot).unwrap();
        let published = target.release(&original.version).unwrap();
        assert_eq!(published.snapshot_id, id);
        assert_eq!(published.normalizer_version, NORMALIZER_VERSION);
        validate_catalog(&target, &published).unwrap();
        for (path, bytes) in before {
            assert_eq!(fs::read(export.join(path)).unwrap(), bytes);
        }
        assert_eq!(
            Cache::reader(&input)
                .unwrap()
                .release(&original.version)
                .unwrap()
                .snapshot_id,
            original.snapshot_id
        );
    }
}
