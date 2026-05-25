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
        // v2 finding 081 / GET-103: registers the `attune://` URL
        // scheme + the .wav/.m4a/.mp3 file associations declared in
        // tauri.conf.json. The frontend subscribes via
        // `@tauri-apps/plugin-deep-link` and the plugin emits a
        // `deep-link://new-url` event for every received URL or file
        // path the OS forwards to the running app.
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::new_default())
        .setup(|app| {
            // Dev builds launch as raw binaries without a .app bundle, so
            // macOS would otherwise show a blank Dock icon. Set it here.
            app::dock_icon::set_dock_icon();

            // On Linux/Windows we also need to register the URL scheme
            // at runtime so the OS knows to forward `attune://...` URLs
            // back to this process. macOS reads the bundle's Info.plist
            // and does not need the runtime call. Failure to register
            // is non-fatal — deep-link receive still works inside the
            // same instance — so we log and continue.
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!(error = %e, "deep-link register_all failed");
                }
            }
            let _ = app;
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
            commands::transcription::get_recording_language,
            commands::transcription::set_recording_language,
            commands::llm::list_providers,
            commands::llm::set_provider_key,
            commands::llm::delete_provider_key,
            commands::llm::test_provider,
            commands::llm::list_provider_models,
            commands::agents::list_agents,
            commands::agents::run_agent,
            commands::agents::list_agent_runs,
            commands::agents::delete_agent_run,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::tasks::set_task_status,
            commands::memory::list_memories,
            commands::memory::get_memory,
            commands::memory::create_memory,
            commands::memory::update_memory,
            commands::memory::delete_memory,
            commands::memory::purge_memory,
            commands::memory::pin_memory,
            commands::memory::search_memories,
            commands::memory::memory_file_path,
            commands::memory::rebuild_memory_index,
            commands::maintenance::clear_recording_artifacts,
            commands::maintenance::export_vault_snapshot,
            commands::maintenance::purge_old_wav_files,
            commands::webhooks::list_webhooks,
            commands::webhooks::save_webhook,
            commands::webhooks::delete_webhook,
            commands::webhooks::test_webhook,
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
