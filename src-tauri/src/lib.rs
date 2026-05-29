//! Attune Tauri 2 app entry. Sets up the window, registers plugins, and
//! exposes Rust functions to the React frontend as Tauri commands.

mod app;
mod commands;

use tauri::Manager;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            // Mirror the persisted privacy_mode setting into the
            // process-global CloudGuard so the very first network call
            // after launch already honours it. v2 finding 048 / GET-42.
            {
                let state: tauri::State<'_, app::AppState> = app.state();
                let on = state.settings.lock().privacy_mode;
                attune_core::cloud_guard::set_airgap(on);
                tracing::info!(privacy_mode = on, "cloud guard initialised");
            }
            // Menu bar (system tray) icon + menu. v2 finding 006 /
            // GET-25. Cited by 8 lenses — recording must be ambient.
            if let Err(e) = app::tray::install(app.handle()) {
                tracing::warn!(error = %e, "tray install failed");
            }
            // NSVisualEffectView vibrancy on the main window. v2
            // finding 011 / GET-45.
            for window in app.webview_windows().values() {
                app::vibrancy::install_window_vibrancy(window);
            }
            // Meeting auto-detection watcher. GET-143. Polls
            // NSWorkspace for conferencing apps and surfaces the HUD.
            app::meeting_watcher::spawn(app.handle().clone());
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::ping,
            commands::devices::list_input_devices,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::recording::recording_status,
            commands::recording::create_note,
            commands::recording::rename_note,
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::pause_recording,
            commands::recording::resume_recording,
            commands::folders::list_folders,
            commands::folders::create_folder,
            commands::folders::rename_folder,
            commands::folders::delete_folder,
            commands::folders::set_note_folder,
            commands::library::list_recordings,
            commands::library::search_note_content,
            commands::library::get_recording,
            commands::library::delete_recording,
            commands::library::reveal_in_finder,
            commands::library::share_paths,
            commands::permissions::list_permissions,
            commands::permissions::open_permission_settings,
            commands::permissions::request_calendar_access,
            commands::calendar::list_attendee_suggestions,
            commands::auth::auth_request_signin_code,
            commands::auth::auth_verify_signin_code,
            commands::auth::auth_status,
            commands::auth::auth_logout,
            commands::account::account_get,
            commands::account::account_update,
            commands::account::account_devices,
            commands::account::account_revoke_device,
            commands::account::account_soft_delete,
            commands::referrals::referrals_generate,
            commands::referrals::referrals_me,
            commands::referrals::referrals_redeem,
            commands::settings_sync::settings_sync_pull,
            commands::settings_sync::settings_sync_push,
            commands::tray::set_tray_recording,
            commands::preferences::open_preferences_window,
            commands::meeting::get_pending_meeting,
            commands::meeting::meeting_take_notes,
            commands::meeting::dismiss_meeting_hud,
            commands::meeting::suppress_meeting_app,
            commands::live_notes::save_live_notes,
            commands::live_notes::load_live_notes,
            commands::ask::ask_note,
            commands::ask::ask_library,
            commands::transcription::transcribe_recording,
            commands::vad::run_vad,
            commands::transcription::read_transcript,
            commands::transcription::locate_transcript_span,
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
            commands::agents::list_note_templates,
            commands::agents::set_note_template,
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
            commands::maintenance::generate_weekly_digest,
            commands::maintenance::export_share_bundle,
            commands::maintenance::git_sync_vault,
            commands::maintenance::git_vault_is_repo,
            commands::maintenance::list_inbox_entries,
            commands::maintenance::archive_inbox_entry,
            commands::maintenance::get_showcase,
            commands::maintenance::save_showcase,
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
