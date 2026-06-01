//! Personal note folders ("Spaces"), GET-162.
//!
//! A folder is just a name. Membership is stored per-note in
//! `<session_dir>/folder.txt` (a single trimmed line); the set of
//! folder names lives in `<output_dir>/folders.json` (an ordered JSON
//! array) so an empty folder — one created but not yet assigned any
//! note — still persists across relaunch.
//!
//! The note's `folder.txt` is the source of truth for membership; the
//! registry is the source of truth for "which folders exist." On read
//! we union the two so a note pointing at a folder missing from the
//! registry (e.g. hand-edited on disk) still surfaces it.

use std::path::Path;

use crate::error::{AttuneError, Result};
use crate::storage::atomic_write::atomic_write;

const REGISTRY_FILENAME: &str = "folders.json";
const FOLDER_MARKER: &str = "folder.txt";

/// Read the ordered folder registry from `<output_dir>/folders.json`.
/// Returns an empty list when the file is missing or unparsable —
/// the per-note markers still let `list_folders` recover names.
fn read_registry(output_dir: &Path) -> Vec<String> {
    let raw = match std::fs::read_to_string(output_dir.join(REGISTRY_FILENAME)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

/// Persist the folder registry atomically.
fn write_registry(output_dir: &Path, folders: &[String]) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(folders)
        .map_err(|e| AttuneError::Storage(format!("serialize folders.json: {e}")))?;
    atomic_write(&output_dir.join(REGISTRY_FILENAME), &bytes)
}

/// Read a note's folder assignment from `<session_dir>/folder.txt`.
fn read_note_folder(session_dir: &Path) -> Option<String> {
    crate::storage::session::read_first_line(session_dir, FOLDER_MARKER)
}

/// Case-insensitive membership check.
fn contains_ci(haystack: &[String], needle: &str) -> bool {
    haystack.iter().any(|f| f.eq_ignore_ascii_case(needle))
}

/// Collect distinct folder names referenced by notes under `output_dir`.
fn folders_in_use(output_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(folder) = read_note_folder(&path) {
            if !contains_ci(&out, &folder) {
                out.push(folder);
            }
        }
    }
    out
}

/// List every folder: registry order first, then any in-use folder not
/// in the registry (so hand-edited or orphaned assignments still show).
pub fn list_folders(output_dir: &Path) -> Vec<String> {
    let mut folders = read_registry(output_dir);
    let mut extras: Vec<String> = folders_in_use(output_dir)
        .into_iter()
        .filter(|f| !contains_ci(&folders, f))
        .collect();
    extras.sort_by_key(|f| f.to_lowercase());
    folders.extend(extras);
    folders
}

/// Create a folder. No-op (idempotent) when one with the same name
/// already exists (case-insensitive). Returns the updated folder list.
pub fn create_folder(output_dir: &Path, name: &str) -> Result<Vec<String>> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AttuneError::Storage("folder name is empty".into()));
    }
    let mut folders = read_registry(output_dir);
    if !contains_ci(&folders, trimmed) {
        folders.push(trimmed.to_string());
        write_registry(output_dir, &folders)?;
    }
    Ok(list_folders(output_dir))
}

/// Rename a folder: update the registry entry and rewrite every note's
/// `folder.txt` that pointed at the old name. Returns the updated list.
pub fn rename_folder(output_dir: &Path, from: &str, to: &str) -> Result<Vec<String>> {
    let to = to.trim();
    if to.is_empty() {
        return Err(AttuneError::Storage("new folder name is empty".into()));
    }
    let mut folders = read_registry(output_dir);
    for f in folders.iter_mut() {
        if f.eq_ignore_ascii_case(from) {
            *f = to.to_string();
        }
    }
    if !contains_ci(&folders, to) {
        folders.push(to.to_string());
    }
    write_registry(output_dir, &folders)?;
    reassign_notes(output_dir, from, Some(to))?;
    Ok(list_folders(output_dir))
}

/// Delete a folder: drop it from the registry and clear the assignment
/// on every note filed under it (the notes themselves are untouched).
/// Returns the updated folder list.
pub fn delete_folder(output_dir: &Path, name: &str) -> Result<Vec<String>> {
    let mut folders = read_registry(output_dir);
    folders.retain(|f| !f.eq_ignore_ascii_case(name));
    write_registry(output_dir, &folders)?;
    reassign_notes(output_dir, name, None)?;
    Ok(list_folders(output_dir))
}

/// Assign (or clear) the folder a single note belongs to. An empty /
/// whitespace `folder` clears the assignment. Assigning a brand-new
/// folder name also registers it so it shows up in the sidebar even if
/// this is the first note to use it.
pub fn set_note_folder(output_dir: &Path, session_dir: &Path, folder: Option<&str>) -> Result<()> {
    let trimmed = folder.map(str::trim).filter(|s| !s.is_empty());
    let path = session_dir.join(FOLDER_MARKER);
    match trimmed {
        Some(name) => {
            atomic_write(&path, name.as_bytes())?;
            // Register the name so an assign-creates flow surfaces it.
            let mut folders = read_registry(output_dir);
            if !contains_ci(&folders, name) {
                folders.push(name.to_string());
                write_registry(output_dir, &folders)?;
            }
        }
        None => match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(AttuneError::Storage(format!(
                    "remove {}: {e}",
                    path.display()
                )))
            }
        },
    }
    Ok(())
}

/// Rewrite every note's `folder.txt` matching `from` to point at `to`
/// (Some = rename, None = clear). Used by rename/delete.
fn reassign_notes(output_dir: &Path, from: &str, to: Option<&str>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match read_note_folder(&path) {
            Some(current) if current.eq_ignore_ascii_case(from) => {
                let marker = path.join(FOLDER_MARKER);
                match to {
                    Some(name) => atomic_write(&marker, name.as_bytes())?,
                    None => {
                        if let Err(e) = std::fs::remove_file(&marker) {
                            if e.kind() != std::io::ErrorKind::NotFound {
                                return Err(AttuneError::Storage(format!(
                                    "remove {}: {e}",
                                    marker.display()
                                )));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_note(output_dir: &Path, label: &str) -> std::path::PathBuf {
        let dir = output_dir.join(label);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn create_then_list_roundtrips() {
        let tmp = tempdir().unwrap();
        let out = create_folder(tmp.path(), "  Work  ").unwrap();
        assert_eq!(out, vec!["Work".to_string()]);
        // Idempotent (case-insensitive).
        let out = create_folder(tmp.path(), "work").unwrap();
        assert_eq!(out, vec!["Work".to_string()]);
        assert_eq!(list_folders(tmp.path()), vec!["Work".to_string()]);
    }

    #[test]
    fn empty_name_rejected() {
        let tmp = tempdir().unwrap();
        assert!(create_folder(tmp.path(), "   ").is_err());
    }

    #[test]
    fn assign_clears_and_registers() {
        let tmp = tempdir().unwrap();
        let note = make_note(tmp.path(), "2026-05-29-a");
        set_note_folder(tmp.path(), &note, Some("Personal")).unwrap();
        assert_eq!(read_note_folder(&note).as_deref(), Some("Personal"));
        // Assigning registered the folder.
        assert!(list_folders(tmp.path()).contains(&"Personal".to_string()));
        // Clearing removes the marker.
        set_note_folder(tmp.path(), &note, None).unwrap();
        assert_eq!(read_note_folder(&note), None);
    }

    #[test]
    fn rename_moves_member_notes() {
        let tmp = tempdir().unwrap();
        let note = make_note(tmp.path(), "2026-05-29-b");
        set_note_folder(tmp.path(), &note, Some("Old")).unwrap();
        rename_folder(tmp.path(), "Old", "New").unwrap();
        assert_eq!(read_note_folder(&note).as_deref(), Some("New"));
        let folders = list_folders(tmp.path());
        assert!(folders.contains(&"New".to_string()));
        assert!(!folders.contains(&"Old".to_string()));
    }

    #[test]
    fn delete_clears_member_notes() {
        let tmp = tempdir().unwrap();
        let note = make_note(tmp.path(), "2026-05-29-c");
        set_note_folder(tmp.path(), &note, Some("Trash")).unwrap();
        delete_folder(tmp.path(), "Trash").unwrap();
        assert_eq!(read_note_folder(&note), None);
        assert!(!list_folders(tmp.path()).contains(&"Trash".to_string()));
    }

    #[test]
    fn list_includes_in_use_not_in_registry() {
        let tmp = tempdir().unwrap();
        let note = make_note(tmp.path(), "2026-05-29-d");
        // Write the marker directly, bypassing the registry.
        atomic_write(&note.join(FOLDER_MARKER), b"Orphan").unwrap();
        assert!(list_folders(tmp.path()).contains(&"Orphan".to_string()));
    }
}
