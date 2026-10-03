//! Souhaits de skins locaux ; le chemin est résolu et les accès sérialisés par l'appelant.

use super::{write_atomic, PreferenceError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

const MAX_WISHES_BYTES: u64 = 128 * 1024;
const MAX_WISHES: usize = 10_000;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WishesDocument {
    version: u32,
    ids: Vec<u32>,
}

/// Seul un fichier absent équivaut à une liste vide ; les versions inconnues sont préservées.
pub fn load_wishes(path: &Path) -> Result<BTreeSet<u32>, PreferenceError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(_) => return Err(PreferenceError::ReadFailed),
    };
    let metadata = file.metadata().map_err(|_| PreferenceError::ReadFailed)?;
    if !metadata.is_file() {
        return Err(PreferenceError::ReadFailed);
    }
    if metadata.len() > MAX_WISHES_BYTES {
        return Err(PreferenceError::TooLarge);
    }
    let mut bytes = Vec::new();
    // Cette seconde borne protège aussi contre un agrandissement après metadata().
    file.take(MAX_WISHES_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PreferenceError::ReadFailed)?;
    if bytes.len() as u64 > MAX_WISHES_BYTES {
        return Err(PreferenceError::TooLarge);
    }
    let document: WishesDocument =
        serde_json::from_slice(&bytes).map_err(|_| PreferenceError::Invalid)?;
    if document.version != 1 || document.ids.contains(&0) {
        return Err(PreferenceError::Invalid);
    }
    if document.ids.len() > MAX_WISHES {
        return Err(PreferenceError::TooLarge);
    }
    let ids: BTreeSet<u32> = document.ids.iter().copied().collect();
    if ids.len() != document.ids.len() {
        return Err(PreferenceError::Invalid);
    }
    Ok(ids)
}

/// Écriture atomique via les primitives natives Windows/macOS ; aucun fichier invalide écrasé.
/// L'appelant doit sérialiser les lectures/modifications/écritures pour un même compte.
pub fn save_wishes(path: &Path, ids: &BTreeSet<u32>) -> Result<(), PreferenceError> {
    if ids.contains(&0) {
        return Err(PreferenceError::Invalid);
    }
    if ids.len() > MAX_WISHES {
        return Err(PreferenceError::TooLarge);
    }
    load_wishes(path)?;
    let bytes = serde_json::to_vec(&WishesDocument {
        version: 1,
        ids: ids.iter().copied().collect(),
    })
    .map_err(|_| PreferenceError::WriteFailed)?;
    if bytes.len() as u64 > MAX_WISHES_BYTES {
        return Err(PreferenceError::TooLarge);
    }
    write_atomic(path, |file| file.write_all(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fichier_absent_renvoie_un_ensemble_vide_sans_creer_de_fichier() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("absent.json");
        assert_eq!(load_wishes(&path), Ok(BTreeSet::new()));
        assert!(!path.exists());
    }

    #[test]
    fn sauvegarde_recharge_et_remplace_en_ordre_deterministe() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("account/wishes.json");
        let ids = BTreeSet::from([103015, 103001, 222003]);
        save_wishes(&path, &ids).unwrap();
        assert_eq!(load_wishes(&path), Ok(ids));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"version":1,"ids":[103001,103015,222003]}"#
        );
        save_wishes(&path, &BTreeSet::new()).unwrap();
        assert_eq!(load_wishes(&path), Ok(BTreeSet::new()));
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn deux_chemins_de_compte_restent_isoles() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.json");
        let second = directory.path().join("second.json");
        save_wishes(&first, &BTreeSet::from([1])).unwrap();
        save_wishes(&second, &BTreeSet::from([2])).unwrap();
        assert_eq!(load_wishes(&first), Ok(BTreeSet::from([1])));
        assert_eq!(load_wishes(&second), Ok(BTreeSet::from([2])));
    }

    #[test]
    fn corruption_schema_et_valeurs_invalides_ne_sont_jamais_ecrases() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("wishes.json");
        for content in [
            "{",
            "null",
            "{}",
            r#"{"version":2,"ids":[1]}"#,
            r#"{"version":0,"ids":[1]}"#,
            r#"{"version":1,"ids":[1],"extra":true}"#,
            r#"{"version":1,"ids":[0]}"#,
            r#"{"version":1,"ids":[-1]}"#,
            r#"{"version":1,"ids":[1.5]}"#,
            r#"{"version":1,"ids":[4294967296]}"#,
            r#"{"version":1,"ids":["1"]}"#,
            r#"{"version":1,"ids":[1,1]}"#,
        ] {
            fs::write(&path, content).unwrap();
            assert_eq!(
                load_wishes(&path),
                Err(PreferenceError::Invalid),
                "{content}"
            );
            assert_eq!(
                save_wishes(&path, &BTreeSet::from([3])),
                Err(PreferenceError::Invalid),
                "{content}"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
    }

    #[test]
    fn borne_la_taille_du_fichier_a_128_kio() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("wishes.json");
        let mut bytes = br#"{"version":1,"ids":[]}"#.to_vec();
        bytes.resize(128 * 1024, b' ');
        fs::write(&path, &bytes).unwrap();
        assert_eq!(load_wishes(&path), Ok(BTreeSet::new()));
        bytes.push(b' ');
        fs::write(&path, &bytes).unwrap();
        assert_eq!(load_wishes(&path), Err(PreferenceError::TooLarge));
        assert_eq!(
            save_wishes(&path, &BTreeSet::new()),
            Err(PreferenceError::TooLarge)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn borne_le_nombre_de_souhaits_et_refuse_zero_sans_ecraser() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("wishes.json");
        let valid = (1..=10_000).collect();
        save_wishes(&path, &valid).unwrap();
        assert_eq!(load_wishes(&path), Ok(valid));
        let original = fs::read(&path).unwrap();
        assert_eq!(
            save_wishes(&path, &(1..=10_001).collect()),
            Err(PreferenceError::TooLarge)
        );
        assert_eq!(
            save_wishes(&path, &BTreeSet::from([0, 1])),
            Err(PreferenceError::Invalid)
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::write(
            &path,
            serde_json::to_vec(
                &serde_json::json!({"version":1,"ids":(1..=10_001).collect::<Vec<_>>()}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(load_wishes(&path), Err(PreferenceError::TooLarge));
    }

    #[test]
    fn refuse_un_dossier_a_la_place_du_fichier() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            load_wishes(directory.path()),
            Err(PreferenceError::ReadFailed)
        );
        assert_eq!(
            save_wishes(directory.path(), &BTreeSet::new()),
            Err(PreferenceError::ReadFailed)
        );
    }
}
