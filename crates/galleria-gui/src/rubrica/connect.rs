//! The parameters of `connect` for a profile of the rubrica (plan B5, T4): the one place where the window reads the keys of a
//! profile. They go to the daemon from here and are never in a view.

use super::{Rubrica, RubricaError};
use galleria_ipc::ConnectParams;
use galleria_profile::{ProfileStore, StoreError, to_connect_params};
use uuid::Uuid;

impl Rubrica {
    /// What the CLI would send for this profile: the same function (`to_connect_params`) over the same archives.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotFound`] for a profile that is not there; [`RubricaError::SecretMissing`] when a key the profile
    /// names is not in the archive; the keyring errors (locked, failed) and the others of the store.
    pub fn connect_params(&self, id: Uuid, replace: bool) -> Result<ConnectParams, RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let secrets = self.secrets(&profile.name)?;
        to_connect_params(&profile, &*secrets, replace).map_err(|e| match e {
            StoreError::Invalid(_) => RubricaError::SecretMissing,
            other => self.fail(other),
        })
    }
}
