//! The command that tells the window where its secrets go (plan B5, T3). It asks the session bus whether the keyring is
//! there and nothing else: no collection is opened, no secret is read, and nothing is unlocked (spec §9, rule 5).

use crate::rubrica::Rubrica;
use galleria_profile::StoreKind;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

/// `keyring` when a write of a secret goes to the keyring of the system, `file` when the session has none and the secrets
/// go to a file that only the user can read. The window shows a fixed notice for the second.
#[derive(Debug, Clone, Serialize)]
pub struct SecretStoreStatus {
    pub store: &'static str,
}

impl From<StoreKind> for SecretStoreStatus {
    fn from(kind: StoreKind) -> Self {
        Self {
            store: match kind {
                StoreKind::Keyring => "keyring",
                StoreKind::File => "file",
            },
        }
    }
}

#[tauri::command(async)]
pub fn secret_store_status(rubrica: State<'_, Arc<Rubrica>>) -> SecretStoreStatus {
    rubrica.secret_store_kind().into()
}
