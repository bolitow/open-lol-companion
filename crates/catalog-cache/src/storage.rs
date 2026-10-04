use crate::*;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
/// Le descripteur garde le verrou interprocessus jusqu’à la fin de l’opération.
pub struct Lease {
    _file: File,
}

pub struct Cache {
    root: PathBuf,
    _lock: Option<File>,
}
fn directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(Error::Invalid);
    }
    Ok(())
}
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit {
        return Err(Error::Invalid);
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Invalid);
    }
    Ok(bytes)
}
fn write_atomic(root: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let target = root.join(name);
    if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::Invalid);
    }
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(target).map_err(|e| Error::Storage(e.error))?;
    #[cfg(unix)]
    File::open(root)?.sync_all()?;
    Ok(())
}
impl Cache {
    /// Lecture seule des objets immuables ; ne bloque pas une synchronisation.
    pub fn reader(root: &Path) -> Result<Self> {
        for p in [
            root.to_path_buf(),
            root.join("objects"),
            root.join("snapshots"),
            root.join("heads"),
        ] {
            let m = fs::symlink_metadata(p)?;
            if !m.is_dir() || m.file_type().is_symlink() {
                return Err(Error::Invalid);
            }
        }
        Ok(Self {
            root: root.into(),
            _lock: None,
        })
    }
    pub fn open(root: &Path) -> Result<Self> {
        directory(root)?;
        for sub in ["objects", "snapshots", "heads", "leases"] {
            directory(&root.join(sub))?;
        }
        let lockpath = root.join("cache.lock");
        if fs::symlink_metadata(&lockpath).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid);
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lockpath)?;
        fs2::FileExt::try_lock_exclusive(&lock)?;
        Ok(Self {
            root: root.into(),
            _lock: Some(lock),
        })
    }
    /// Une fenêtre garde cette garde tant qu’elle peut encore lire cet instantané.
    pub fn lease(&self, id: &str) -> Result<Lease> {
        if !valid_id(id) {
            return Err(Error::Invalid);
        }
        let path = self.root.join("leases").join(id);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid);
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        fs2::FileExt::lock_shared(&file)?;
        self.manifest(id)?;
        Ok(Lease { _file: file })
    }
    /// Journal de reprise écrit avant les octets. Jamais considéré comme actif.
    pub fn prepare(&mut self, m: &Manifest) -> Result<()> {
        validate_manifest(m)?;
        write_atomic(
            &self.root.join("snapshots"),
            &m.snapshot_id,
            &serde_json::to_vec(m)?,
        )?;
        write_atomic(&self.root, "pending", m.snapshot_id.as_bytes())
    }
    /// Appelé seulement sous le verrou d’écriture desktop ; les lecteurs épinglés sont épargnés.
    pub fn cleanup(&mut self) -> Result<()> {
        use std::collections::BTreeSet;
        let mut keep: BTreeSet<_> = self.pointers().into_iter().collect();
        if let Ok(b) = read_bounded(&self.root.join("pending"), 64) {
            if let Ok(id) = String::from_utf8(b) {
                if valid_id(&id) {
                    keep.insert(id);
                }
            }
        }
        let mut referenced = BTreeSet::new();
        for entry in fs::read_dir(self.root.join("snapshots"))? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if !valid_id(&id) {
                continue;
            }
            let lockpath = self.root.join("leases").join(&id);
            if fs::symlink_metadata(&lockpath).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(Error::Invalid);
            }
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lockpath)?;
            let unused = fs2::FileExt::try_lock_exclusive(&lock).is_ok();
            if keep.contains(&id) || !unused {
                // Si une référence conservée est corrompue, différer le ramasse-miettes.
                let m = self.manifest(&id)?;
                referenced.extend(m.files.values().map(|f| f.sha256.clone()));
            } else {
                fs::remove_file(entry.path())?;
            }
        }
        for entry in fs::read_dir(self.root.join("objects"))? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if valid_id(&id) && !referenced.contains(&id) {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
    pub fn object_path(&self, id: &str) -> PathBuf {
        self.root.join("objects").join(id)
    }
    pub fn has(&self, f: &FileEntry) -> bool {
        valid_id(&f.sha256)
            && read_bounded(&self.object_path(&f.sha256), f.bytes)
                .is_ok_and(|b| b.len() as u64 == f.bytes && digest(&b) == f.sha256)
    }
    pub fn put(&mut self, f: &FileEntry, bytes: &[u8]) -> Result<()> {
        if !valid_id(&f.sha256)
            || bytes.len() as u64 != f.bytes
            || f.bytes > MAX_JSON
            || digest(bytes) != f.sha256
        {
            return Err(Error::Invalid);
        }
        if !self.has(f) {
            write_atomic(&self.root.join("objects"), &f.sha256, bytes)?;
        }
        Ok(())
    }
    pub fn manifest(&self, id: &str) -> Result<Manifest> {
        if !valid_id(id) {
            return Err(Error::Invalid);
        }
        let m: Manifest = serde_json::from_slice(&read_bounded(
            &self.root.join("snapshots").join(id),
            MAX_MANIFEST as u64,
        )?)?;
        validate_manifest(&m)?;
        if m.snapshot_id != id {
            return Err(Error::Invalid);
        }
        Ok(m)
    }
    pub fn read(&self, id: &str, path: &str) -> Result<Vec<u8>> {
        let m = self.manifest(id)?;
        let f = m.files.get(path).ok_or(Error::Invalid)?;
        let bytes = read_bounded(&self.object_path(&f.sha256), f.bytes)?;
        if bytes.len() as u64 != f.bytes || digest(&bytes) != f.sha256 {
            return Err(Error::Invalid);
        }
        Ok(bytes)
    }
    pub fn verify(&self, m: &Manifest) -> Result<()> {
        validate_manifest(m)?;
        if m.files.values().any(|f| !self.has(f)) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn save(&mut self, m: &Manifest) -> Result<()> {
        self.verify(m)?;
        write_atomic(
            &self.root.join("snapshots"),
            &m.snapshot_id,
            &serde_json::to_vec(m)?,
        )
    }
    /// Publication serveur : le pointeur de release ne change qu’après validation complète.
    pub fn publish(&mut self, m: &Manifest) -> Result<()> {
        self.save(m)?;
        write_atomic(
            &self.root.join("heads"),
            &m.version,
            m.snapshot_id.as_bytes(),
        )
    }
    pub fn release(&self, version: &str) -> Result<Manifest> {
        if version_parts(version).is_none() {
            return Err(Error::Invalid);
        }
        let id = String::from_utf8(read_bounded(&self.root.join("heads").join(version), 64)?)
            .map_err(|_| Error::Invalid)?;
        let m = self.manifest(&id)?;
        if m.version != version {
            return Err(Error::Invalid);
        }
        Ok(m)
    }
    pub fn activate(&mut self, m: &Manifest) -> Result<()> {
        self.save(m)?;
        let old = self.active()?.map(|v| v.snapshot_id);
        let ids = if old.as_deref() == Some(&m.snapshot_id) {
            self.pointers()
        } else {
            std::iter::once(m.snapshot_id.clone()).chain(old).collect()
        };
        write_atomic(&self.root, "active.json", &serde_json::to_vec(&ids)?)
    }
    fn pointers(&self) -> Vec<String> {
        read_bounded(&self.root.join("active.json"), 1024)
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
            .filter(|v| v.len() <= 2 && v.iter().all(|id| valid_id(id)))
            .unwrap_or_default()
    }
    pub fn recovered(&self, id: Option<&str>) -> bool {
        self.root.join("active.json").exists()
            && (id.is_none() || self.pointers().first().map(String::as_str) != id)
    }
    pub fn active(&self) -> Result<Option<Manifest>> {
        for id in self.pointers() {
            if let Ok(m) = self.manifest(&id) {
                if self.verify(&m).is_ok() {
                    return Ok(Some(m));
                }
            }
        }
        Ok(None)
    }
}
