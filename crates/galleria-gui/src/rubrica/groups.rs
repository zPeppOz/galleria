//! The groups of the rubrica (plan B3, T6). A group is a name (Q30): who belongs to it is `Profile.group`, and
//! `galleria.toml` keeps the order of the groups and the groups that have nobody in them. The phase 1 reads neither
//! more nor less than before: it never looked at `galleria.toml` apart from creating it.
//!
//! Renaming or deleting a group rewrites more than one file and cannot be atomic. The order is: **the profiles first,
//! the list last**. A failure in between leaves some profiles with the new name and the list with the old one, and the
//! same call finishes the work: the old name is still in the list, so it is found, and the profiles that already moved
//! are not in the way. The secrets are never opened.

use super::{Rubrica, RubricaError, stamp};
use crate::edit::{GROUP_MAX, check_label};
use galleria_profile::{ProfileStore, RubricaSettings};

/// The name as it will be kept (no spaces at the sides), or why it is not a name.
fn clean(name: &str) -> Result<String, RubricaError> {
    let name = name.trim();
    match check_label(name, GROUP_MAX) {
        Some(reason) => Err(RubricaError::GroupInvalid(reason)),
        None => Ok(name.to_owned()),
    }
}

/// The same name for the purpose of «unique»: the case does not tell two groups apart.
fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

impl Rubrica {
    fn open_for_groups(&self) -> Result<(ProfileStore, RubricaSettings), RubricaError> {
        let store = ProfileStore::open(self.dir()).map_err(|e| self.fail(e))?;
        let settings = store.settings().map_err(|e| self.fail(e))?;
        Ok((store, settings))
    }

    /// Puts every profile of the group `from` in the group `to` (`None`: no group). Each one is written with a new time
    /// of modification, so a window that holds it learns that it changed.
    fn move_members(
        &self,
        store: &ProfileStore,
        from: &str,
        to: Option<&str>,
    ) -> Result<(), RubricaError> {
        let (profiles, _) = store.list_lenient().map_err(|e| self.fail(e))?;
        for mut profile in profiles
            .into_iter()
            .filter(|p| p.group.as_deref() == Some(from))
        {
            profile.group = to.map(str::to_owned);
            if let Some(now) = stamp() {
                profile.modified = now;
            }
            store.save(&profile).map_err(|e| self.fail(e))?;
        }
        Ok(())
    }

    fn write_groups(
        &self,
        store: &ProfileStore,
        mut settings: RubricaSettings,
        groups: Vec<String>,
    ) -> Result<Vec<String>, RubricaError> {
        settings.groups = groups;
        store.save_settings(&settings).map_err(|e| self.fail(e))?;
        Ok(settings.groups)
    }

    /// Adds a group at the end of the list. A name that only the profiles have (a file written by hand, the CLI) is not
    /// in the list yet, so this is also how the window adds it.
    ///
    /// # Errors
    ///
    /// [`RubricaError::GroupInvalid`] for a name that is empty, too long or with control characters,
    /// [`RubricaError::GroupExists`] when the list has it already (whatever the case).
    pub fn group_create(&self, name: &str) -> Result<Vec<String>, RubricaError> {
        let name = clean(name)?;
        let (store, settings) = self.open_for_groups()?;
        if settings.groups.iter().any(|g| same(g, &name)) {
            return Err(RubricaError::GroupExists);
        }
        let mut groups = settings.groups.clone();
        groups.push(name);
        self.write_groups(&store, settings, groups)
    }

    /// Gives the group `from` a new name, in the same place of the list, and moves its profiles to it.
    ///
    /// # Errors
    ///
    /// [`RubricaError::GroupNotFound`] when `from` is not in the list, the errors of [`Rubrica::group_create`] for `to`
    /// (it may differ from `from` by case only: that is the same group).
    pub fn group_rename(&self, from: &str, to: &str) -> Result<Vec<String>, RubricaError> {
        let to = clean(to)?;
        let (store, settings) = self.open_for_groups()?;
        let Some(at) = settings.groups.iter().position(|g| g == from) else {
            return Err(RubricaError::GroupNotFound);
        };
        if settings
            .groups
            .iter()
            .enumerate()
            .any(|(i, g)| i != at && same(g, &to))
        {
            return Err(RubricaError::GroupExists);
        }
        if from == to {
            return Ok(settings.groups);
        }
        self.move_members(&store, from, Some(&to))?;
        let mut groups = settings.groups.clone();
        groups[at] = to;
        self.write_groups(&store, settings, groups)
    }

    /// Takes a group out of the list. Its profiles stay, without a group.
    ///
    /// # Errors
    ///
    /// [`RubricaError::GroupNotFound`] when it is not in the list.
    pub fn group_delete(&self, name: &str) -> Result<Vec<String>, RubricaError> {
        let (store, settings) = self.open_for_groups()?;
        let Some(at) = settings.groups.iter().position(|g| g == name) else {
            return Err(RubricaError::GroupNotFound);
        };
        self.move_members(&store, name, None)?;
        let mut groups = settings.groups.clone();
        groups.remove(at);
        self.write_groups(&store, settings, groups)
    }

    /// Puts the groups in this order, which must be the same groups as the list has now, each once.
    ///
    /// # Errors
    ///
    /// [`RubricaError::GroupInvalid`] with the reason `not_a_permutation` when it is not.
    pub fn groups_reorder(&self, names: &[String]) -> Result<Vec<String>, RubricaError> {
        let (store, settings) = self.open_for_groups()?;
        let (mut wanted, mut have) = (names.to_vec(), settings.groups.clone());
        wanted.sort();
        have.sort();
        if wanted != have {
            return Err(RubricaError::GroupInvalid("not_a_permutation"));
        }
        self.write_groups(&store, settings, names.to_vec())
    }
}
