use attune_core::briefs::MeetingBrief;
use attune_core::llm::{KeyStore, ProviderId};
use attune_core::storage::SettingsStore;
use tauri::{Emitter, Manager, State};
use tracing::info;

use crate::app::meeting_watcher::{DetectedMeeting, MEETING_HUD_LABEL};
use crate::app::AppState;

const MAIN_WINDOW_LABEL: &str = "main";

const TAKE_NOTES_EVENT: &str = "meeting:take-notes";

#[tauri::command]
pub fn get_pending_meeting(state: State<'_, AppState>) -> Option<DetectedMeeting> {
    state.pending_meeting.lock().clone()
}

#[tauri::command]
pub fn meeting_take_notes(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    *state.pending_meeting.lock() = None;

    if let Some(main) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
        let _ = main.emit(TAKE_NOTES_EVENT, ());
    } else {
        let _ = app.emit(TAKE_NOTES_EVENT, ());
    }

    close_hud(&app);
    info!("meeting take-notes triggered from HUD");
    Ok(())
}

#[tauri::command]
pub fn dismiss_meeting_hud(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    *state.pending_meeting.lock() = None;
    close_hud(&app);
    Ok(())
}

#[tauri::command]
pub async fn suppress_meeting_app(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    bundle_id: String,
) -> Result<(), String> {
    let (settings_to_save, path) = {
        let mut s = state.settings.lock();
        if !s.notification_muted_apps.iter().any(|m| m == &bundle_id) {
            s.notification_muted_apps.push(bundle_id.clone());
        }
        (s.clone(), state.settings_store.path().to_path_buf())
    };
    *state.pending_meeting.lock() = None;

    let to_persist = settings_to_save.clone();
    tauri::async_runtime::spawn_blocking(move || SettingsStore::new(path).save(&to_persist))
        .await
        .map_err(|e| format!("suppress_meeting_app task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    *state.settings.lock() = settings_to_save;
    close_hud(&app);
    info!(bundle_id, "muted auto-detect for app from HUD");
    Ok(())
}

fn close_hud(app: &tauri::AppHandle) {
    if let Some(hud) = app.get_webview_window(MEETING_HUD_LABEL) {
        let _ = hud.close();
    }
}

#[tauri::command]
pub async fn get_meeting_brief(
    state: State<'_, AppState>,
    attendees: Vec<String>,
) -> Result<Option<MeetingBrief>, String> {
    if attendees.is_empty() {
        return Ok(None);
    }

    let (output_dir, memory_dir, privacy) = {
        let s = state.settings.lock();
        (s.output_dir.clone(), s.memory_dir.clone(), s.privacy_mode)
    };

    if privacy {
        return Ok(None);
    }

    let api_key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(ProviderId::OpenAi))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    let Some(api_key) = api_key else {
        return Ok(None);
    };

    let memory_store = state.memory_store()?;
    let brief = attune_core::briefs::generate(
        &attendees,
        &output_dir,
        &memory_store,
        &api_key,
        "gpt-4o-mini",
    )
    .await;

    info!(
        has_brief = brief.is_some(),
        attendees = attendees.len(),
        "meeting brief request completed"
    );
    let _ = memory_dir;
    Ok(brief)
}
