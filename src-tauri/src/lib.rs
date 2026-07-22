mod app;
mod commands;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::Manager;
use tracing_subscriber::EnvFilter;

static WATCHER_STOP: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();
static WATCHER_HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

use crate::app::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new_default())
        .setup(|app| {
            app::dock_icon::set_dock_icon();

            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!(error = %e, "deep-link register_all failed");
                }
            }

            {
                let state: tauri::State<'_, app::AppState> = app.state();
                let settings = state.settings.lock().clone();
                let on = settings.privacy_mode;
                folio_core::cloud_guard::set_airgap(on);
                let vault_root = settings
                    .output_dir
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| settings.output_dir.clone());
                let policy = folio_core::cloud_guard::load_egress_policy(&vault_root);
                folio_core::cloud_guard::set_egress_policy(policy);
                tracing::info!(privacy_mode = on, "cloud guard initialised");
            }

            if let Err(e) = app::tray::install(app.handle()) {
                tracing::warn!(error = %e, "tray install failed");
            }

            for window in app.webview_windows().values() {
                app::vibrancy::install_window_vibrancy(window);
            }

            let (watcher_handle, watcher_stop) = app::meeting_watcher::spawn(app.handle().clone());
            let _ = WATCHER_STOP.set(watcher_stop);
            if let Ok(mut slot) = WATCHER_HANDLE.lock() {
                *slot = Some(watcher_handle);
            }

            app::sync_scheduler::spawn(app.handle().clone());
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::ping,
            commands::devices::list_input_devices,
            commands::devices::check_mic_level,
            commands::devices::start_mic_monitor,
            commands::devices::stop_mic_monitor,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::server::remote_register,
            commands::server::remote_login,
            commands::server::remote_logout,
            commands::server::remote_me,
            commands::server::test_remote_endpoint,
            commands::server::sync_recording,
            commands::server::get_sync_status,
            commands::recording::recording_status,
            commands::recording::create_note,
            commands::recording::rename_note,
            commands::recording::get_enhanced_notes_accepted,
            commands::recording::set_enhanced_notes_accepted,
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
            commands::chats::list_chat_threads,
            commands::chats::save_chat_thread,
            commands::chats::delete_chat_thread,
            commands::library::get_recording,
            commands::library::delete_recording,
            commands::library::export_note_markdown,
            commands::library::reveal_in_finder,
            commands::library::share_paths,
            commands::permissions::list_permissions,
            commands::permissions::open_permission_settings,
            commands::permissions::request_permission,
            commands::permissions::request_calendar_access,
            commands::calendar::list_attendee_suggestions,
            commands::calendar::calendar_authorization_status,
            commands::calendar::next_calendar_event,
            commands::calendar::list_calendar_events,
            commands::tray::set_tray_recording,
            commands::recording_bar::show_recording_bar,
            commands::recording_bar::hide_recording_bar,
            commands::recording_bar::recording_bar_stop,
            commands::recording_bar::recording_bar_pause,
            commands::recording_bar::recording_bar_resume,
            commands::preferences::open_preferences_window,
            commands::meeting::get_pending_meeting,
            commands::meeting::meeting_take_notes,
            commands::meeting::dismiss_meeting_hud,
            commands::meeting::suppress_meeting_app,
            commands::meeting::get_meeting_brief,
            commands::live_notes::save_live_notes,
            commands::live_notes::load_live_notes,
            commands::ask::ask_note,
            commands::ask::ask_library,
            commands::ask::ask_folder,
            commands::transcription::transcribe_recording,
            commands::transcription::diarize_session,
            commands::vad::run_vad,
            commands::transcription::read_transcript,
            commands::transcription::locate_transcript_span,
            commands::transcription::locate_note_evidence,
            commands::transcription::save_transcript,
            commands::transcription::whisper_model_status,
            commands::transcription::ensure_whisper_model,
            commands::diarization::diarization_model_status,
            commands::diarization::ensure_diarization_models,
            commands::speakers::list_session_speakers,
            commands::speakers::rename_session_speaker,
            commands::speakers::confirm_session_speaker,
            commands::speakers::reject_session_speaker,
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
            commands::maintenance::generate_weekly_digest,
            commands::maintenance::export_share_bundle,
            commands::maintenance::git_sync_vault,
            commands::maintenance::git_vault_is_repo,
            commands::maintenance::list_inbox_entries,
            commands::maintenance::archive_inbox_entry,
            commands::maintenance::get_showcase,
            commands::maintenance::save_showcase,
            commands::maintenance::apply_cross_track_aec,
            commands::recipes::list_recipes,
            commands::mcp_connect::generate_mcp_config,
            commands::mcp_connect::write_mcp_config,
            commands::mcp_grants::list_mcp_grants,
            commands::mcp_grants::grant_mcp_client,
            commands::mcp_grants::revoke_mcp_client,
            commands::mcp_grants::list_mcp_access_log,
            commands::mcp_grants::check_mcp_grant,
            commands::mcp_grants::record_mcp_access,
            commands::webhooks::list_webhooks,
            commands::webhooks::save_webhook,
            commands::webhooks::delete_webhook,
            commands::webhooks::test_webhook,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(stop) = WATCHER_STOP.get() {
                    stop.store(true, Ordering::Relaxed);
                }
                if let Ok(mut slot) = WATCHER_HANDLE.lock() {
                    if let Some(handle) = slot.take() {
                        match handle.join() {
                            Ok(()) => {}
                            Err(_) => tracing::warn!("meeting-watcher thread panicked"),
                        }
                    }
                }

                if let Some(state) = app.try_state::<app::AppState>() {
                    state.stop_live_transcript();
                    state.join_live_transcript();
                }
            }
        });
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
