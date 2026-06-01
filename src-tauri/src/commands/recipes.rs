//! User-authored chat recipe loader (GET-194).

use tauri::State;

use attune_core::recipes::UserRecipe;

use crate::app::AppState;

/// List recipes from `.attune/recipes/*.toml` in the vault root.
/// Returns the built-in recipes are merged on the frontend; this
/// command returns only the user-authored additions.
#[tauri::command]
pub fn list_recipes(state: State<'_, AppState>) -> Vec<UserRecipe> {
    let output_dir = state.settings.lock().output_dir.clone();
    let vault_root = output_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(output_dir);
    attune_core::recipes::load(&vault_root)
}
