pub(crate) use crate::sys::{atomic, check, directory, read_private};
use crate::{FileSecretStore, Profile, SecretStore};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Profile not found")]
    NotFound,
    #[error("Ambiguous profile name: {0:?}")]
    Ambiguous(Vec<(Uuid, String)>),
    #[error("Profile created by a newer version (schema {found})")]
    NewerSchema { found: u32 },
    #[error("{0}")]
    Invalid(String),
    #[error("Profile storage I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error(
        "Insecure permissions on {path}; mode {mode:o}; use chmod 600 for files or chmod 700 for directories"
    )]
    InsecurePermissions { path: PathBuf, mode: u32 },
    #[error("Wrong owner of {path}; restore ownership to the current user")]
    WrongOwner { path: PathBuf },
    /// The keyring is there but locked, or the unlocking was refused. Not the same as «no keyring».
    #[error("The keyring is locked")]
    KeyringLocked,
    /// Any other error of the keyring service. The text never holds a value.
    #[error("Keyring failed: {0}")]
    Keyring(String),
}
/// A file of the profiles folder that [`ProfileStore::list_lenient`] could not use.
#[derive(Debug)]
pub struct Skipped {
    pub file_name: String,
    pub error: StoreError,
}
fn read_profile(path: &Path) -> Result<Profile, StoreError> {
    let p = Profile::from_toml(&read_private(path)?)?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(p.id.to_string().as_str()) {
        return Err(StoreError::Invalid(
            "Profile ID does not match its filename".into(),
        ));
    }
    Ok(p)
}
#[derive(Clone)]
pub struct ProfileStore {
    dir: PathBuf,
}
impl ProfileStore {
    pub fn default_dir() -> PathBuf {
        crate::sys::default_dir()
    }
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        directory(dir)?;
        directory(&dir.join("profiles"))?;
        let settings = dir.join("galleria.toml");
        if !settings.try_exists()? {
            atomic(&settings, "schema_version = 1\n")?;
        }
        Ok(Self { dir: dir.into() })
    }
    pub fn list(&self) -> Result<Vec<Profile>, StoreError> {
        let mut result = vec![];
        for entry in fs::read_dir(self.dir.join("profiles"))? {
            let path = entry?.path();
            if path.extension().is_some_and(|s| s == "toml") {
                let p = Profile::from_toml(&read_private(&path)?)?;
                if path.file_stem().and_then(|s| s.to_str()) != Some(p.id.to_string().as_str()) {
                    return Err(StoreError::Invalid(
                        "Profile ID does not match its filename".into(),
                    ));
                }
                result.push(p);
            }
        }
        result.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(result)
    }
    pub(crate) fn root(&self) -> &Path {
        &self.dir
    }
    /// Like [`list`](Self::list), but a profile that cannot be read does not hide the others: it comes back in the
    /// second list with the name of its file and the reason. The folder itself is still checked and still fails
    /// the whole call (permissions, owner, not a folder): that is a problem of the store, not of one profile.
    pub fn list_lenient(&self) -> Result<(Vec<Profile>, Vec<Skipped>), StoreError> {
        let dir = self.dir.join("profiles");
        check(&dir, &fs::symlink_metadata(&dir)?)?;
        let (mut profiles, mut skipped) = (vec![], vec![]);
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().is_none_or(|s| s != "toml") {
                continue;
            }
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            match read_profile(&path) {
                Ok(p) => profiles.push(p),
                Err(error) => skipped.push(Skipped { file_name, error }),
            }
        }
        profiles.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        skipped.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        Ok((profiles, skipped))
    }
    pub fn find(&self, name_or_id: &str) -> Result<Profile, StoreError> {
        if let Ok(id) = Uuid::parse_str(name_or_id) {
            let path = self.dir.join(format!("profiles/{id}.toml"));
            return match read_private(&path) {
                Ok(s) => {
                    let p = Profile::from_toml(&s)?;
                    if p.id != id {
                        return Err(StoreError::Invalid("Profile ID mismatch".into()));
                    }
                    Ok(p)
                }
                Err(StoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    Err(StoreError::NotFound)
                }
                Err(e) => Err(e),
            };
        }
        let mut found: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|p| p.name == name_or_id)
            .collect();
        if found.len() > 1 {
            return Err(StoreError::Ambiguous(
                found.into_iter().map(|p| (p.id, p.name)).collect(),
            ));
        }
        found.pop().ok_or(StoreError::NotFound)
    }
    pub fn save(&self, p: &Profile) -> Result<(), StoreError> {
        directory(&self.dir.join("profiles"))?;
        atomic(
            &self.dir.join(format!("profiles/{}.toml", p.id)),
            &p.to_toml()?,
        )
    }
    pub fn delete(&self, id: &Uuid) -> Result<(), StoreError> {
        FileSecretStore::open(&self.dir)?.delete_all(id)?;
        fs::remove_file(self.dir.join(format!("profiles/{id}.toml")))?;
        Ok(())
    }
}
