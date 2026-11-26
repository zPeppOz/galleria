//! The parameters of `connect` for a profile of the rubrica (plan B5, T4) and the credentials the user asked to remember (T6): the
//! one place where the window reads the keys of a profile. They go to the daemon from here and are never in a view.

use super::{Rubrica, RubricaError};
use galleria_ipc::{BackendParams, ConnectParams, Secret};
use galleria_profile::{ProfileStore, StoreError, to_connect_params};
use uuid::Uuid;

/// The two keys of the archive that «Remember» writes (spec §9, rule 4), and the names the profile lists for them (Q46).
const USERNAME: &str = "openvpn.username";
const PASSWORD: &str = "openvpn.password";

impl Rubrica {
    /// What the CLI would send for this profile: the same function (`to_connect_params`) over the same archives, and, when the
    /// profile names remembered credentials, the user and the password as well. A credential that the profile names and the archive
    /// no longer has is not an error (P5): it is not sent, the daemon asks for it, and the names go out of the profile.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotFound`] for a profile that is not there; [`RubricaError::SecretMissing`] when a key the profile
    /// names is not in the archive; the keyring errors (locked, failed) and the others of the store.
    pub fn connect_params(&self, id: Uuid, replace: bool) -> Result<ConnectParams, RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let secrets = self.secrets(&profile.name)?;
        let mut params = to_connect_params(&profile, &*secrets, replace).map_err(|e| match e {
            StoreError::Invalid(_) => RubricaError::SecretMissing,
            other => self.fail(other),
        })?;
        let remembered = [USERNAME, PASSWORD]
            .iter()
            .all(|name| profile.secret_refs.iter().any(|r| r == name));
        if let (true, BackendParams::Openvpn(openvpn)) = (remembered, &mut params.backend) {
            let user = secrets.get(&id, USERNAME).map_err(|e| self.fail(e))?;
            let password = secrets.get(&id, PASSWORD).map_err(|e| self.fail(e))?;
            match (user, password) {
                (Some(user), Some(password)) => {
                    openvpn.username = Some(user.expose().to_owned());
                    openvpn.password = Some(password);
                }
                // what is left of them goes too, so that the profile does not claim what it does not have
                _ => {
                    let _ = self.forget_credentials(id);
                }
            }
        }
        Ok(params)
    }

    /// Keeps the user and the password of a connection that worked, in the archive of the secrets, and names them in the profile.
    /// What `connected` was waiting for has happened by now: only then does anything of what was typed reach the archive.
    ///
    /// # Errors
    ///
    /// The errors of the store and of the keyring (locked, failed); on one of them nothing stays written.
    pub fn remember_credentials(
        &self,
        id: Uuid,
        username: &Secret,
        password: &Secret,
    ) -> Result<(), RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let mut profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let secrets = self.secrets(&profile.name)?;
        secrets
            .set(&id, USERNAME, username)
            .map_err(|e| self.fail(e))?;
        if let Err(error) = secrets.set(&id, PASSWORD, password) {
            let _ = secrets.delete(&id, USERNAME);
            return Err(self.fail(error));
        }
        let missing: Vec<&str> = [USERNAME, PASSWORD]
            .into_iter()
            .filter(|name| !profile.secret_refs.iter().any(|r| r == name))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        profile
            .secret_refs
            .extend(missing.into_iter().map(str::to_owned));
        // `modified` stays: the list is not reordered by a connection
        if let Err(error) = store.save(&profile) {
            let _ = secrets.delete(&id, USERNAME);
            let _ = secrets.delete(&id, PASSWORD);
            return Err(self.fail(error));
        }
        Ok(())
    }

    /// Takes the remembered user and password out of the archive and their names out of the profile. The keys of the profile
    /// (`openvpn.block.*`) stay. What is not there is not an error.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotFound`] for a profile that is not there; the errors of the store and of the keyring. The names stay in the
    /// profile when the keys could not be taken out.
    pub fn forget_credentials(&self, id: Uuid) -> Result<(), RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let mut profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let secrets = self.secrets(&profile.name)?;
        for key in [USERNAME, PASSWORD] {
            secrets.delete(&id, key).map_err(|e| self.fail(e))?;
        }
        let before = profile.secret_refs.len();
        profile
            .secret_refs
            .retain(|name| name != USERNAME && name != PASSWORD);
        if profile.secret_refs.len() != before {
            store.save(&profile).map_err(|e| self.fail(e))?;
        }
        Ok(())
    }
}
