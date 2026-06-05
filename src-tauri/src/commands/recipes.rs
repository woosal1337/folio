use tauri::State;

use folio_core::recipes::UserRecipe;

use crate::app::AppState;

#[tauri::command]
pub fn list_recipes(state: State<'_, AppState>) -> Vec<UserRecipe> {
    let output_dir = state.settings.lock().output_dir.clone();
    let vault_root = output_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(output_dir);
    folio_core::recipes::load(&vault_root)
}
