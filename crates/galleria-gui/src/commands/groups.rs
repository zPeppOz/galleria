//! Commands of the groups (P4: four small operations instead of «replace everything»). Thin: the work is in
//! `rubrica/groups.rs`. Each one answers with the new list of groups, in order. They take names and nothing else: no
//! path, no id (spec §3.2, rule 3).

use crate::{error::CmdError, rubrica::Rubrica};
use std::sync::Arc;
use tauri::State;

/// Adds a group at the end of the list (also how the window adds one that only the profiles have).
#[tauri::command(async)]
pub fn group_create(
    rubrica: State<'_, Arc<Rubrica>>,
    name: String,
) -> Result<Vec<String>, CmdError> {
    Ok(rubrica.group_create(&name)?)
}

/// Renames a group in its place and moves its profiles to the new name.
#[tauri::command(async)]
pub fn group_rename(
    rubrica: State<'_, Arc<Rubrica>>,
    from: String,
    to: String,
) -> Result<Vec<String>, CmdError> {
    Ok(rubrica.group_rename(&from, &to)?)
}

/// Takes a group out of the list; its profiles stay, without a group.
#[tauri::command(async)]
pub fn group_delete(
    rubrica: State<'_, Arc<Rubrica>>,
    name: String,
) -> Result<Vec<String>, CmdError> {
    Ok(rubrica.group_delete(&name)?)
}

/// Puts the groups in this order, which has to be a permutation of the list.
#[tauri::command(async)]
pub fn groups_reorder(
    rubrica: State<'_, Arc<Rubrica>>,
    names: Vec<String>,
) -> Result<Vec<String>, CmdError> {
    Ok(rubrica.groups_reorder(&names)?)
}
