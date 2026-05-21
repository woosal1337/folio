//! Attune Tauri 2 app entry. Sets up the window, registers plugins, and
//! exposes Rust functions to the React frontend as Tauri commands.

mod commands;
mod state;

use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::list_input_devices,
            commands::get_settings,
            commands::save_settings,
            commands::recording_status,
            commands::start_recording,
            commands::stop_recording,
            commands::list_recordings,
            commands::reveal_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,cpal=warn,reqwest=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
