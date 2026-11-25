use crate::{
    ProfileStore, StoreError,
    store::{atomic, read_private},
};
use serde::{Deserialize, Serialize};
/// The version of `galleria.toml` this build writes and understands.
const SCHEMA_OF_SETTINGS: u32 = 1;
/// `galleria.toml`: what the rubrica keeps besides the profiles. For now only the order of the groups (spec §6).
/// `schema_version = 1` alone is a valid file: it is what `ProfileStore::open` writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubricaSettings {
    pub schema_version: u32,
    #[serde(default)]
    pub groups: Vec<String>,
}
impl ProfileStore {
    /// Reads `galleria.toml`. A newer schema is reported as such, an unknown key is an error (same strictness as the
    /// profiles), and the permissions are checked like for every file of the store.
    pub fn settings(&self) -> Result<RubricaSettings, StoreError> {
        let text = read_private(&self.root().join("galleria.toml"))?;
        let value: toml::Value = toml::from_str(&text)
            .map_err(|_| StoreError::Invalid("Invalid galleria.toml".into()))?;
        if let Some(n) = value
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            && n > i64::from(SCHEMA_OF_SETTINGS)
        {
            return Err(StoreError::NewerSchema {
                found: u32::try_from(n)
                    .map_err(|_| StoreError::Invalid("Invalid schema version".into()))?,
            });
        }
        let settings: RubricaSettings = value
            .try_into()
            .map_err(|_| StoreError::Invalid("Invalid or unknown galleria.toml key".into()))?;
        if settings.schema_version != SCHEMA_OF_SETTINGS {
            return Err(StoreError::Invalid("Invalid galleria.toml schema".into()));
        }
        Ok(settings)
    }
}
impl ProfileStore {
    /// Writes `galleria.toml`: a rename of a private temporary file, so a reader sees the old file or the new one and
    /// never half of it, with the permissions of the store (`0600`). It does not overwrite a file written by a newer
    /// version (`NewerSchema`, the file is left as it is), and it does not write settings of a schema other than the one
    /// this build understands.
    pub fn save_settings(&self, settings: &RubricaSettings) -> Result<(), StoreError> {
        match settings.schema_version.cmp(&SCHEMA_OF_SETTINGS) {
            std::cmp::Ordering::Greater => {
                return Err(StoreError::NewerSchema {
                    found: settings.schema_version,
                });
            }
            std::cmp::Ordering::Less => {
                return Err(StoreError::Invalid("Invalid galleria.toml schema".into()));
            }
            std::cmp::Ordering::Equal => {}
        }
        // What is there now: a newer schema, a key this version does not know or a file that does not read all stop the write.
        self.settings()?;
        let text = toml::to_string(settings)
            .map_err(|_| StoreError::Invalid("Cannot encode galleria.toml".into()))?;
        atomic(&self.root().join("galleria.toml"), &text)
    }
}
