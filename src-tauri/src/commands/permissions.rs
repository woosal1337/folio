//! TCC permission walkthrough commands. v2 finding 003 / GET-31.
//!
//! Empty or blocked permissions are the silent killer of first-
//! recording. This module exposes the rows the walkthrough screen
//! renders — Microphone, Screen Recording, Calendar, Notifications —
//! with a rationale per bucket and an Open-System-Settings deep link.
//!
//! Detecting the live TCC status per bucket requires per-API FFI
//! calls (AVCaptureDevice authorizationStatusForMediaType,
//! CGPreflightScreenCaptureAccess, EKEventStore, UNUserNotificationCenter).
//! Those land in a macOS-gated follow-up. For now every status
//! comes back `Unknown`; the UI still renders the rows with their
//! rationale and working Open-Settings buttons.

use attune_core::permissions::{Permission, PermissionRow, PermissionStatus};

const MIC_URL: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";
const SCREEN_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
const CALENDAR_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";
const NOTIFICATIONS_URL: &str = "x-apple.systempreferences:com.apple.preference.notifications";

const MIC_RATIONALE: &str =
    "We record what you say. Without microphone access, your half of every meeting is silent.";
const SCREEN_RATIONALE: &str =
    "We record what the other side says by capturing system audio. Screen Recording is the macOS API that allows it.";
const CALENDAR_RATIONALE: &str =
    "Pre-fills meeting titles and attendees on Stop, and back-fills the calendar event's notes with the summary.";
const NOTIFICATIONS_RATIONALE: &str =
    "Used only for 'recording started' / 'summary ready' alerts. Disabled features stay disabled.";

#[tauri::command]
pub fn list_permissions() -> Vec<PermissionRow> {
    vec![
        PermissionRow {
            permission: Permission::Microphone,
            status: PermissionStatus::Unknown,
            rationale: MIC_RATIONALE.to_string(),
            settings_url: MIC_URL.to_string(),
        },
        PermissionRow {
            permission: Permission::ScreenRecording,
            status: PermissionStatus::Unknown,
            rationale: SCREEN_RATIONALE.to_string(),
            settings_url: SCREEN_URL.to_string(),
        },
        PermissionRow {
            permission: Permission::Calendar,
            status: PermissionStatus::Unknown,
            rationale: CALENDAR_RATIONALE.to_string(),
            settings_url: CALENDAR_URL.to_string(),
        },
        PermissionRow {
            permission: Permission::Notifications,
            status: PermissionStatus::Unknown,
            rationale: NOTIFICATIONS_RATIONALE.to_string(),
            settings_url: NOTIFICATIONS_URL.to_string(),
        },
    ]
}

#[tauri::command]
pub fn open_permission_settings(
    app: tauri::AppHandle,
    permission: Permission,
) -> Result<(), String> {
    let url = match permission {
        Permission::Microphone => MIC_URL,
        Permission::ScreenRecording => SCREEN_URL,
        Permission::Calendar => CALENDAR_URL,
        Permission::Notifications => NOTIFICATIONS_URL,
    };
    open_url(&app, url)
}

/// GET-128 stub. Trigger the EKEventStore TCC prompt by deep-linking
/// into System Settings → Privacy & Security → Calendar. A true
/// `EKEventStore.requestFullAccessToEvents` FFI call needs the
/// objc2-event-kit binding to land in attune-core; until then this
/// is the user-equivalent path. Returns `Ok(())` so the UI can flip
/// into "granting" state regardless of whether the user actually
/// toggles Attune on — Settings → Calendar will report the
/// authoritative status next time the user opens it.
#[tauri::command]
pub fn request_calendar_access(app: tauri::AppHandle) -> Result<(), String> {
    open_url(&app, CALENDAR_URL)
}

#[cfg(target_os = "macos")]
fn open_url(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn open_url(_app: &tauri::AppHandle, _url: &str) -> Result<(), String> {
    Err("open_permission_settings is only supported on macOS".into())
}
