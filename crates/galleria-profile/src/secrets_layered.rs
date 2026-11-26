use crate::{SecretStore, StoreError};
use galleria_ipc::Secret;
use uuid::Uuid;

/// Where a write of the archive goes when the keyring is available.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Writes {
    /// The window: the keyring, and the file only when there is no keyring.
    Keyring,
    /// The CLI: always the file.
    File,
}

/// Where a write ends up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StoreKind {
    Keyring,
    File,
}

/// The keyring and the file as one archive (spec §9). Reads look in the keyring first and then in the file; a locked
/// keyring is an error, never a reason to read the file. Writes follow [`Writes`]. After a write to the keyring the
/// copy in the file is removed, so a key never has two values.
pub struct LayeredSecretStore {
    keyring: Option<Box<dyn SecretStore>>,
    file: Box<dyn SecretStore>,
    writes: Writes,
}

impl LayeredSecretStore {
    pub fn new(
        keyring: Option<Box<dyn SecretStore>>,
        file: Box<dyn SecretStore>,
        writes: Writes,
    ) -> Self {
        Self {
            keyring,
            file,
            writes,
        }
    }
    /// Where a write goes: `Keyring` only when `writes` is `Keyring` and there is a keyring.
    pub fn kind(&self) -> StoreKind {
        match (&self.keyring, self.writes) {
            (Some(_), Writes::Keyring) => StoreKind::Keyring,
            _ => StoreKind::File,
        }
    }
}

impl SecretStore for LayeredSecretStore {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        if let Some(keyring) = &self.keyring
            && let Some(value) = keyring.get(id, key)?
        {
            return Ok(Some(value));
        }
        self.file.get(id, key)
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        match (&self.keyring, self.writes) {
            (Some(keyring), Writes::Keyring) => {
                keyring.set(id, key, value)?;
                // The write is done and the keyring wins on reads: a copy that stays in the file is only untidy.
                let _ = self.file.delete(id, key);
                Ok(())
            }
            _ => self.file.set(id, key, value),
        }
    }
    fn delete(&self, id: &Uuid, key: &str) -> Result<(), StoreError> {
        let in_keyring = self.keyring.as_ref().map(|k| k.delete(id, key));
        self.file.delete(id, key)?;
        in_keyring.unwrap_or(Ok(()))
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        let in_keyring = self.keyring.as_ref().map(|k| k.delete_all(id));
        self.file.delete_all(id)?;
        in_keyring.unwrap_or(Ok(()))
    }
}
