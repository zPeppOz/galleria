use crate::{Cmd, NetError, plan::interface};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub undo: Vec<Cmd>,
    pub interfaces: Vec<String>,
}
impl Journal {
    pub fn write(&self, path: &Path) -> Result<(), NetError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        if bytes.len() > 1_048_576 {
            return Err(NetError::InvalidConfig("journal too large"));
        }
        let parent = path
            .parent()
            .ok_or(NetError::InvalidConfig("journal has no parent"))?;
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&bytes)?;
        file.flush()?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| NetError::Journal(e.error))?;
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Option<Self>, NetError> {
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
            Ok(meta) if !meta.is_file() => {
                return Err(NetError::InvalidConfig("journal is not a regular file"));
            }
            Ok(_) => {}
        }
        let mut bytes = Vec::new();
        File::open(path)?.take(1_048_577).read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            return Err(NetError::InvalidConfig("journal too large"));
        }
        let journal: Self = serde_json::from_slice(&bytes)
            .map_err(|_| NetError::InvalidConfig("invalid journal"))?;
        journal.validate()?;
        Ok(Some(journal))
    }
    fn validate(&self) -> Result<(), NetError> {
        if !self.interfaces.iter().all(|s| interface(s)) {
            return Err(NetError::InvalidConfig(
                "journal names an unrelated interface",
            ));
        }
        for cmd in &self.undo {
            cmd.validate_undo()?;
        }
        Ok(())
    }
}
pub(crate) fn remove(path: &Path) -> Result<(), NetError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
