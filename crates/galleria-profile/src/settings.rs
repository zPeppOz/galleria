use crate::{ProfileStore, StoreError, store::read_private};
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
