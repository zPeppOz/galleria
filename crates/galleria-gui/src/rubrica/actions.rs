//! Duplicate, export and delete (plan B3, T8). The secrets are read and written here, in Rust, and never leave it: what a
//! command returns has none, and what «Export» writes goes from the archive to the file without passing through the
//! webview.

use super::{ProfileSummary, Rubrica, RubricaError, stamp, summarize};
use crate::edit::{FieldIssue, NAME_MAX, check_label};
use crate::export::{SavePicker, suggested_name, write_export};
use galleria_ipc::ProfileRef;
use galleria_profile::{ProfileStore, StoreError, export_config};
use uuid::Uuid;

impl Rubrica {
    /// A copy of the profile under a new id and a name the window gives: the same settings, group, notes and text, a new
    /// time of creation and modification, **not a favourite**, and **all the secrets** the profile keeps, read from the
    /// archive and written under the new id. It is all or nothing: the secrets are written first, and if one of them
    /// cannot be, what was written is wiped and no profile is saved; if the profile cannot be saved, the secrets go.
    ///
    /// # Errors
    ///
    /// [`RubricaError::Invalid`] for a name that is not one (nothing is written), the errors of the store otherwise;
    /// a secret that the profile names and the archive has not is a store error too.
    pub fn duplicate(&self, id: Uuid, name: &str) -> Result<ProfileSummary, RubricaError> {
        let name = name.trim();
        if let Some(code) = check_label(name, NAME_MAX) {
            return Err(RubricaError::Invalid(vec![FieldIssue {
                field: "name".into(),
                code,
            }]));
        }
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let original = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let secrets = self.secrets()?;
        // Everything is read before anything is written.
        let mut values = vec![];
        for key in &original.secret_refs {
            let value = secrets
                .get(&original.id, key)
                .map_err(|e| self.fail(e))?
                .ok_or_else(|| {
                    self.fail(StoreError::Invalid(
                        "Profile is missing required key material".into(),
                    ))
                })?;
            values.push((key, value));
        }
        let mut copy = original.clone();
        copy.id = Uuid::new_v4();
        copy.name = name.to_owned();
        copy.favorite = false;
        if let Some(now) = stamp() {
            copy.created = now.clone();
            copy.modified = now;
        }
        for (key, value) in &values {
            if let Err(error) = secrets.set(&copy.id, key, value) {
                // best effort: the error that matters is the one that stopped the copy
                let _ = secrets.delete_all(&copy.id);
                return Err(self.fail(error));
            }
        }
        if let Err(error) = store.save(&copy) {
            let _ = secrets.delete_all(&copy.id);
            return Err(self.fail(error));
        }
        Ok(summarize(&copy, None))
    }

    /// Writes the profile in its native form, with its keys (Q31), at the place the window «Save» gives. The text is made
    /// first, so that a profile whose secrets are not all there fails before a window is shown for nothing; a cancel
    /// writes nothing and answers `false`. The text goes from the archive to the file, `0600`, and nowhere else.
    ///
    /// # Errors
    ///
    /// The errors of the store, and [`RubricaError::Export`] when the file cannot be written.
    pub fn export(&self, id: Uuid, picker: &dyn SavePicker) -> Result<bool, RubricaError> {
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let profile = store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        let secrets = self.secrets()?;
        let text = export_config(&profile, &*secrets).map_err(|e| self.fail(e))?;
        let Some(path) = picker.pick(&suggested_name(&profile.name, profile.protocol)) else {
            return Ok(false);
        };
        write_export(&path, &text)?;
        Ok(true)
    }

    /// Takes the profile, its secrets and its last use away. `active` is the profile of the session that the service has
    /// open (not `disconnected`): that one is refused, because a session cannot be left without its profile.
    ///
    /// # Errors
    ///
    /// [`RubricaError::InUse`] for the profile of the active session, the errors of the store otherwise (a profile that
    /// is not there is `NotFound`). A last use that cannot be forgotten is not an error: nothing reads it for a profile
    /// that is not there.
    pub fn delete(&self, id: Uuid, active: Option<&ProfileRef>) -> Result<(), RubricaError> {
        if active.is_some_and(|session| session.id == id.to_string()) {
            return Err(RubricaError::InUse);
        }
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        // `find` first: a profile that is not there is `NotFound`, whatever `delete` makes of a missing file
        store.find(&id.to_string()).map_err(|e| self.fail(e))?;
        store.delete(&id).map_err(|e| self.fail(e))?;
        let _ = self.state.forget(id);
        Ok(())
    }
}
