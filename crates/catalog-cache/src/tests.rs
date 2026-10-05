use super::*;
use std::collections::BTreeMap;
fn manifest(bytes: &[u8]) -> Manifest {
    let mut m = Manifest {
        schema_version: 1,
        version: "16.20.1".into(),
        normalizer_version: 3,
        snapshot_id: String::new(),
        files: BTreeMap::from([(
            "test.json".into(),
            FileEntry {
                bytes: bytes.len() as u64,
                sha256: digest(bytes),
                media_type: "application/json".into(),
            },
        )]),
    };
    m.snapshot_id = snapshot_id(&m).unwrap();
    m
}
#[test]
fn manifeste_refuse_chemins_et_corruption() {
    let m = manifest(b"{}");
    assert!(validate_manifest(&m).is_ok());
    let mut bad = m.clone();
    bad.files
        .insert("../outside.json".into(), bad.files["test.json"].clone());
    bad.snapshot_id = snapshot_id(&bad).unwrap();
    assert!(validate_manifest(&bad).is_err());
    let mut bad = m.clone();
    bad.version = "16.21.1".into();
    assert!(validate_manifest(&bad).is_err());
}
#[test]
fn reprise_verifie_octets_et_activation_garde_ancien() {
    let root = tempfile::tempdir().unwrap();
    let mut c = Cache::open(root.path()).unwrap();
    let m = manifest(b"{}");
    assert!(c.activate(&m).is_err());
    c.put(&m.files["test.json"], b"{}").unwrap();
    assert!(c.has(&m.files["test.json"]));
    c.activate(&m).unwrap();
    assert_eq!(c.active().unwrap().unwrap().snapshot_id, m.snapshot_id);
    let n = manifest(b"{\"new\":true}");
    assert!(c.activate(&n).is_err());
    assert_eq!(c.active().unwrap().unwrap().snapshot_id, m.snapshot_id);
    c.put(&n.files["test.json"], b"{\"new\":true}").unwrap();
    c.activate(&n).unwrap();
    assert_eq!(c.read(&m.snapshot_id, "test.json").unwrap(), b"{}");
    std::fs::write(c.object_path(&n.files["test.json"].sha256), b"corrupt").unwrap();
    assert_eq!(c.active().unwrap().unwrap().snapshot_id, m.snapshot_id);
}
#[test]
fn verrou_est_exclusif_et_libere_a_la_sortie() {
    let root = tempfile::tempdir().unwrap();
    let cache = Cache::open(root.path()).unwrap();
    assert!(Cache::open(root.path()).is_err());
    drop(cache);
    assert!(Cache::open(root.path()).is_ok());
}
#[test]
fn aucune_release_future_ou_autre_patch() {
    let versions = vec!["16.21.1".into(), "16.20.2".into(), "16.20.1".into()];
    assert_eq!(select_release("16.20", &versions), Some("16.20.2".into()));
    assert_eq!(select_release("16.19", &versions), None);
}
#[test]
fn un_paquet_sans_index_n_est_pas_un_catalogue_activable() {
    let root = tempfile::tempdir().unwrap();
    let mut c = Cache::open(root.path()).unwrap();
    let m = manifest(b"{}");
    c.put(&m.files["test.json"], b"{}").unwrap();
    c.save(&m).unwrap();
    assert!(validate_catalog(&c, &m).is_err());
}
#[test]
fn bornes_et_extensions_sont_verifiees() {
    for path in [
        "/test.json",
        "a//b.json",
        "a/../b.json",
        "C:test.json",
        "a\\b.json",
        "x.exe",
    ] {
        let mut m = manifest(b"{}");
        let f = m.files.remove("test.json").unwrap();
        m.files.insert(path.into(), f);
        m.snapshot_id = snapshot_id(&m).unwrap();
        assert!(validate_manifest(&m).is_err(), "{path}");
    }
    let mut m = manifest(b"{}");
    m.files.get_mut("test.json").unwrap().bytes = MAX_JSON + 1;
    m.snapshot_id = snapshot_id(&m).unwrap();
    assert!(validate_manifest(&m).is_err());
}
fn catalog_fixture() -> (tempfile::TempDir, Cache, Manifest) {
    use serde_json::json;
    let mut files = BTreeMap::new();
    files.insert(
        "champions.json".into(),
        serde_json::to_vec(&json!({"103":{"key":"Ahri","fr":"Ahri","en":"Ahri"}})).unwrap(),
    );
    files.insert("champion-directory.json".into(),serde_json::to_vec(&json!({"version":"16.20.1","champions":[{"id":103,"key":"Ahri","names":{"fr":"Ahri","en":"Ahri"},"titles":{"fr":"Renarde","en":"Fox"},"categories":["Mage"],"image":"champions/103.png"}]})).unwrap());
    files.insert("champions/103.png".into(), b"image".to_vec());
    for locale in ["fr_FR", "en_US"] {
        let record = |kind: &str, id: &str| json!({"kind":kind,"id":id,"locale":locale,"namespace":"standard","name":"Ahri","description":null,"icon":null,"fields":{},"stats":{},"effects":[],"coverage":{}});
        files.insert(format!("catalog/{locale}.json"),serde_json::to_vec(&json!({"version":"16.20.1","records":[record("item","1"),record("rune","2"),record("rune_shard","3"),record("summoner_spell","4")]})).unwrap());
        let mut records = vec![record("champion", "103")];
        for slot in ["Q", "W", "E", "R", "passive"] {
            records.push(record("ability", &format!("103:{slot}")));
        }
        files.insert(
            format!("catalog/champions/103/{locale}.json"),
            serde_json::to_vec(&json!({"version":"16.20.1","records":records})).unwrap(),
        );
    }
    let root = tempfile::tempdir().unwrap();
    let mut cache = Cache::open(root.path()).unwrap();
    let mut m = Manifest {
        schema_version: 1,
        version: "16.20.1".into(),
        normalizer_version: 3,
        snapshot_id: String::new(),
        files: BTreeMap::new(),
    };
    for (path, bytes) in files {
        let f = FileEntry {
            bytes: bytes.len() as u64,
            sha256: digest(&bytes),
            media_type: if path.ends_with(".png") {
                "image/png"
            } else {
                "application/json"
            }
            .into(),
        };
        cache.put(&f, &bytes).unwrap();
        m.files.insert(path, f);
    }
    m.snapshot_id = snapshot_id(&m).unwrap();
    cache.save(&m).unwrap();
    (root, cache, m)
}
#[test]
fn refuse_avant_activation_les_champs_non_lisibles_par_le_front() {
    for (path, pointer, replacement) in [
        (
            "catalog/fr_FR.json",
            "/records/0/description",
            serde_json::json!(42),
        ),
        (
            "champion-directory.json",
            "/champions/0/categories",
            serde_json::json!(["nonsense"]),
        ),
    ] {
        let (_root, mut cache, mut m) = catalog_fixture();
        assert!(validate_catalog(&cache, &m).is_ok());
        let mut v: serde_json::Value =
            serde_json::from_slice(&cache.read(&m.snapshot_id, path).unwrap()).unwrap();
        *v.pointer_mut(pointer).unwrap() = replacement;
        let bytes = serde_json::to_vec(&v).unwrap();
        let f = m.files.get_mut(path).unwrap();
        f.bytes = bytes.len() as u64;
        f.sha256 = digest(&bytes);
        cache.put(f, &bytes).unwrap();
        m.snapshot_id = snapshot_id(&m).unwrap();
        cache.save(&m).unwrap();
        assert!(validate_catalog(&cache, &m).is_err(), "{pointer}");
    }
}
#[test]
fn nettoyage_garde_les_lectures_epinglees_et_le_telechargement_incomplet() {
    let root = tempfile::tempdir().unwrap();
    let mut c = Cache::open(root.path()).unwrap();
    let a = manifest(b"{\"a\":1}");
    c.put(&a.files["test.json"], b"{\"a\":1}").unwrap();
    c.activate(&a).unwrap();
    let lease = c.lease(&a.snapshot_id).unwrap();
    let b = manifest(b"{\"b\":1}");
    c.put(&b.files["test.json"], b"{\"b\":1}").unwrap();
    c.activate(&b).unwrap();
    let d = manifest(b"{\"d\":1}");
    c.put(&d.files["test.json"], b"{\"d\":1}").unwrap();
    c.activate(&d).unwrap();
    let pending = manifest(b"{\"pending\":1}");
    c.prepare(&pending).unwrap();
    c.put(&pending.files["test.json"], b"{\"pending\":1}")
        .unwrap();
    c.cleanup().unwrap();
    assert!(c.read(&a.snapshot_id, "test.json").is_ok());
    assert!(c.has(&pending.files["test.json"]));
    drop(lease);
    c.cleanup().unwrap();
    assert!(c.manifest(&a.snapshot_id).is_err());
    assert!(!c.has(&a.files["test.json"]));
    assert!(c.read(&b.snapshot_id, "test.json").is_ok());
}

#[test]
fn manifeste_json_refuse_les_chemins_dupliques() {
    let m = manifest(b"{}");
    let raw = serde_json::to_string(&m).unwrap();
    let entry = serde_json::to_string(&m.files["test.json"]).unwrap();
    let duplicate = raw.replace(
        &format!("\"test.json\":{entry}"),
        &format!("\"test.json\":{entry},\"test.json\":{entry}"),
    );
    assert!(serde_json::from_str::<Manifest>(&duplicate).is_err());
}
#[test]
fn le_repli_apres_corruption_est_explicitement_signale() {
    let root = tempfile::tempdir().unwrap();
    let mut cache = Cache::open(root.path()).unwrap();
    assert!(!cache.recovered(None));
    let m = manifest(b"{}");
    cache.put(&m.files["test.json"], b"{}").unwrap();
    cache.activate(&m).unwrap();
    assert!(!cache.recovered(Some(&m.snapshot_id)));
    std::fs::write(root.path().join("active.json"), b"broken").unwrap();
    assert!(cache.recovered(None));
}

#[test]
fn champs_nullable_obligatoires_ne_peuvent_pas_etre_absents() {
    for field in ["description", "icon"] {
        let (_root, mut cache, mut m) = catalog_fixture();
        let path = "catalog/fr_FR.json";
        let mut v: serde_json::Value =
            serde_json::from_slice(&cache.read(&m.snapshot_id, path).unwrap()).unwrap();
        v["records"][0].as_object_mut().unwrap().remove(field);
        let bytes = serde_json::to_vec(&v).unwrap();
        let f = m.files.get_mut(path).unwrap();
        f.bytes = bytes.len() as u64;
        f.sha256 = digest(&bytes);
        cache.put(f, &bytes).unwrap();
        m.snapshot_id = snapshot_id(&m).unwrap();
        cache.save(&m).unwrap();
        assert!(validate_catalog(&cache, &m).is_err(), "{field}");
    }
}
