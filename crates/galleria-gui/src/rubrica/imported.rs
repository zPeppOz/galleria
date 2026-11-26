//! The save of an import (plan B4, T1): the one place that writes what an import found, and only after «Import». The
//! order is the one of the CLI and of «Duplicate»: the secrets first, then the profile, and all or nothing.

use super::{ProfileSummary, Rubrica, RubricaError, summarize};
use crate::edit::{FieldIssue, NAME_MAX, check_label};
use galleria_profile::{Imported, ProfileStore};

impl Rubrica {
    /// Saves the imported profile under `name` and in `group` (`None`: no group): its secrets go into the archive of the
    /// secrets, then the profile into its folder. If a secret cannot be written, what was written is wiped and no profile is
    /// saved; if the profile cannot be saved, the secrets go. The name and the group are checked before anything is
    /// opened for writing: the folder of the profiles is not even created for a name that is refused.
    ///
    /// # Errors
    ///
    /// [`RubricaError::Invalid`] for a name that is not one, [`RubricaError::GroupNotFound`] for a group that is not in the
    /// list (an import does not make groups), the errors of the store otherwise.
    pub fn commit_import(
        &self,
        imported: &Imported,
        name: &str,
        group: Option<&str>,
    ) -> Result<ProfileSummary, RubricaError> {
        let name = name.trim();
        if let Some(code) = check_label(name, NAME_MAX) {
            return Err(RubricaError::Invalid(vec![FieldIssue {
                field: "name".into(),
                code,
            }]));
        }
        if let Some(group) = group
            && !self.list()?.groups.iter().any(|g| g == group)
        {
            return Err(RubricaError::GroupNotFound);
        }
        let mut profile = imported.profile.clone();
        profile.name = name.to_owned();
        profile.group = group.map(str::to_owned);
        let store = ProfileStore::open(&self.dir).map_err(|e| self.fail(e))?;
        let secrets = self.secrets(name)?;
        for (key, value) in &imported.secrets {
            if let Err(error) = secrets.set(&profile.id, key, value) {
                // best effort: the error that matters is the one that stopped the import
                let _ = secrets.delete_all(&profile.id);
                return Err(self.fail(error));
            }
        }
        if let Err(error) = store.save(&profile) {
            let _ = secrets.delete_all(&profile.id);
            return Err(self.fail(error));
        }
        Ok(summarize(&profile, None))
    }
}
