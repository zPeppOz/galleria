use crate::{
    StoreError,
    store::{atomic, check, directory, read_private},
};
use galleria_ipc::Secret;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zeroize::Zeroizing;
pub trait SecretStore {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError>;
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError>;
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError>;
}
pub struct FileSecretStore {
    dir: PathBuf,
}
impl FileSecretStore {
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        directory(dir)?;
        let dir = dir.join("secrets");
        directory(&dir)?;
        Ok(Self { dir })
    }
    fn load(&self, id: &Uuid) -> Result<BTreeMap<String, Secret>, StoreError> {
        // Revalidate on every access; never repair permissions silently.
        let meta = fs::symlink_metadata(&self.dir)?;
        check(&self.dir, &meta)?;
        let text = match read_private(&self.dir.join(format!("{id}.toml"))) {
            Ok(s) => Zeroizing::new(s),
            Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(e) => return Err(e),
        };
        toml::from_str(&text).map_err(|_| StoreError::Invalid("Invalid secrets TOML".into()))
    }
}
impl SecretStore for FileSecretStore {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        Ok(self.load(id)?.remove(key))
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        let mut values = self.load(id)?;
        values.insert(key.into(), value.clone());
        let text = Zeroizing::new(
            toml::to_string(&values)
                .map_err(|_| StoreError::Invalid("Cannot encode secrets".into()))?,
        );
        atomic(&self.dir.join(format!("{id}.toml")), &text)
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        check(&self.dir, &fs::symlink_metadata(&self.dir)?)?;
        match fs::remove_file(self.dir.join(format!("{id}.toml"))) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
