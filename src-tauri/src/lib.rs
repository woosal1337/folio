//! Attune Tauri 2 app entry. Sets up the window, registers plugins, and
//! exposes Rust functions to the React frontend as Tauri commands.

mod app;
mod commands;

use tracing_subscriber::EnvFilter;

use crate::app::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new_default())
        .setup(|_app| {
            // Dev builds launch as raw binaries without a .app bundle, so
            // macOS would otherwise show a blank Dock icon. Set it here.
            app::dock_icon::set_dock_icon();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::ping,
            commands::devices::list_input_devices,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::recording::recording_status,
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::library::list_recordings,
            commands::library::get_recording,
            commands::library::delete_recording,
            commands::library::reveal_in_finder,
            commands::transcription::transcribe_recording,
            commands::transcription::read_transcript,
            commands::transcription::save_transcript,
            commands::transcription::whisper_model_status,
            commands::transcription::ensure_whisper_model,
            commands::llm::list_providers,
            commands::llm::set_provider_key,
            commands::llm::delete_provider_key,
            commands::llm::test_provider,
            commands::llm::list_provider_models,
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
