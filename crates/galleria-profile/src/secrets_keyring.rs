//! The keyring of the desktop as an archive of the secrets (spec §9): the Secret Service of the session bus, through
//! `dbus-secret-service` (plan B5, Q44), with a session that does not encrypt: the bus is the user's own, and whoever
//! can listen to it can already ask the keyring for the secret.
//!
//! Every call opens its own short connection, so the store holds no bus connection and can be shared between threads.
//! Nothing here unlocks the keyring by itself: only [`SecretStore::get`], `set` and `delete*`, which a gesture of the
//! user leads to, can ask for an unlock; [`keyring_present`] and [`KeyringSecretStore::open`] never do.
use crate::{SecretStore, StoreError};
use dbus_secret_service::{EncryptionType, Error, Item, SecretService};
use galleria_ipc::Secret;
use std::{collections::HashMap, path::Path, time::Duration};
use uuid::Uuid;
use zeroize::Zeroize;

const SERVICE: &str = "galleria";
const CONTENT_TYPE: &str = "text/plain";
/// How long the user has to answer a request to unlock.
const PROMPT_SECONDS: u64 = 60;
/// How long the session bus has to answer the first call.
const BUS_SECONDS: u64 = 2;
const IS_LOCKED: &str = "org.freedesktop.Secret.Error.IsLocked";

/// True when the session bus has a Secret Service (`org.freedesktop.secrets`, or one that can be started) **with a default
/// collection** to keep the secrets in. A service with none (GNOME Keyring on Ubuntu 26.04 with an automatic login: nobody made
/// a keyring yet, and making one is a window that asks for a password) is no keyring for the app: it is treated like a bus
/// without the service, the secrets go to the file with the warning (D25, F1 of the daily round) and nothing stops on
/// «The keyring did not answer». A default collection that is **locked** is a keyring: it asks for the unlock, as before. It
/// opens no collection and unlocks nothing. Without a session bus it answers at once.
pub fn keyring_present() -> bool {
    has_session_bus()
        && SecretService::connect_with_connection_timeout(
            EncryptionType::Plain,
            Duration::from_secs(BUS_SECONDS),
        )
        .is_ok_and(|service| service.get_default_collection().is_ok())
}

/// libdbus would try to start a bus of its own (`dbus-launch`) when it finds no address; here no address is no keyring.
fn has_session_bus() -> bool {
    std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some_and(|a| !a.is_empty())
        || std::env::var_os("XDG_RUNTIME_DIR")
            .is_some_and(|dir| Path::new(&dir).join("bus").exists())
}

pub struct KeyringSecretStore {
    prompt_seconds: u64,
    profile_name: Option<String>,
}

impl KeyringSecretStore {
    /// `None` when the bus or the service are not there. Unlocks nothing.
    pub fn open() -> Option<Self> {
        keyring_present().then_some(Self {
            prompt_seconds: PROMPT_SECONDS,
            profile_name: None,
        })
    }
    /// The label «Galleria: <profile name> (<key>)» for the items that `set` creates from here on.
    pub fn labelled(mut self, profile_name: &str) -> Self {
        self.profile_name = Some(profile_name.into());
        self
    }
    /// How long a request to unlock waits for the user (60 s by default; 0 never asks).
    pub fn with_prompt_timeout(mut self, seconds: u64) -> Self {
        self.prompt_seconds = seconds;
        self
    }
    fn connect(&self) -> Result<SecretService, StoreError> {
        SecretService::connect_with_max_prompt_timeout(EncryptionType::Plain, self.prompt_seconds)
            .map_err(failed)
    }
    /// The items of the profile (and of the key, when given). A locked one is unlocked first: it is a request that
    /// the user's gesture led to. A keyring that is locked as a whole may hide its items: then it is unlocked too.
    fn find<'a>(
        &self,
        service: &'a SecretService,
        id: &Uuid,
        key: Option<&str>,
    ) -> Result<Vec<Item<'a>>, StoreError> {
        let id = id.to_string();
        let mut attributes = HashMap::from([("service", SERVICE), ("profile", id.as_str())]);
        if let Some(key) = key {
            attributes.insert("key", key);
        }
        let found = service.search_items(attributes.clone()).map_err(failed)?;
        let mut items = found.unlocked;
        if !found.locked.is_empty() {
            let locked: Vec<&Item<'_>> = found.locked.iter().collect();
            service.unlock_all(&locked).map_err(failed)?;
            items.extend(found.locked);
        }
        if items.is_empty() {
            match service.get_default_collection() {
                Ok(collection) if collection.is_locked().map_err(failed)? => {
                    collection.unlock().map_err(failed)?;
                    return Ok(service.search_items(attributes).map_err(failed)?.unlocked);
                }
                Ok(_) | Err(Error::NoResult) => {}
                Err(e) => return Err(failed(e)),
            }
        }
        Ok(items)
    }
    fn label(&self, id: &Uuid, key: &str) -> String {
        match &self.profile_name {
            Some(name) => format!("Galleria: {name} ({key})"),
            None => format!("Galleria: {id} ({key})"),
        }
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError> {
        let service = self.connect()?;
        let Some(item) = self.find(&service, id, Some(key))?.into_iter().next() else {
            return Ok(None);
        };
        let bytes = item.get_secret().map_err(failed)?;
        match String::from_utf8(bytes) {
            Ok(text) => Ok(Some(Secret::new(text))),
            Err(e) => {
                e.into_bytes().zeroize();
                Err(StoreError::Keyring("the value is not text".into()))
            }
        }
    }
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError> {
        let service = self.connect()?;
        let collection = match service.get_default_collection() {
            Ok(collection) => collection,
            Err(Error::NoResult) => {
                return Err(StoreError::Keyring("no default collection".into()));
            }
            Err(e) => return Err(failed(e)),
        };
        collection.ensure_unlocked().map_err(failed)?;
        let id_text = id.to_string();
        let attributes = HashMap::from([
            ("service", SERVICE),
            ("profile", id_text.as_str()),
            ("key", key),
        ]);
        collection
            .create_item(
                &self.label(id, key),
                attributes,
                value.expose().as_bytes(),
                true,
                CONTENT_TYPE,
            )
            .map_err(failed)?;
        Ok(())
    }
    fn delete(&self, id: &Uuid, key: &str) -> Result<(), StoreError> {
        let service = self.connect()?;
        for item in self.find(&service, id, Some(key))? {
            item.delete().map_err(failed)?;
        }
        Ok(())
    }
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError> {
        let service = self.connect()?;
        for item in self.find(&service, id, None)? {
            item.delete().map_err(failed)?;
        }
        Ok(())
    }
}

/// The errors of the crate say what happened and never what was stored, but their text can quote the answer of the
/// service: only the name of the D-Bus error is kept.
fn failed(error: Error) -> StoreError {
    let what = match error {
        Error::Locked | Error::Prompt => return StoreError::KeyringLocked,
        Error::Dbus(e) => match e.name() {
            Some(IS_LOCKED) => return StoreError::KeyringLocked,
            Some(name) => format!("D-Bus error {name}"),
            None => "D-Bus error".into(),
        },
        Error::Unavailable => "no session bus or no Secret Service".into(),
        Error::NoResult => "the service found nothing where it should".into(),
        Error::UnsupportedSecretFormat => "the value is not text".into(),
        Error::Crypto(_) => "cryptography of the session".into(),
        Error::Path(_) | Error::Parse => "answer of the service not understood".into(),
        _ => "unexpected error".into(),
    };
    StoreError::Keyring(what)
}
