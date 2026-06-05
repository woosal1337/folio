use attune_core::permissions::{Permission, PermissionRow, PermissionStatus};

const MIC_URL: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";

const SCREEN_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";

const SYSTEM_AUDIO_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_SystemAudioRecording";
const CALENDAR_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";
const NOTIFICATIONS_URL: &str = "x-apple.systempreferences:com.apple.preference.notifications";

const MIC_RATIONALE: &str =
    "We record what you say. Without microphone access, your half of every meeting is silent.";

const SCREEN_RATIONALE: &str =
    "We record what the other side says by capturing system audio. Screen Recording is the macOS API that allows it.";

const SYSTEM_AUDIO_RATIONALE: &str =
    "We capture what the other side says using the system audio API. \
     This only grants audio access — not screen recording — so Attune appears under \
     System Audio Recording Only in Privacy & Security.";
const CALENDAR_RATIONALE: &str =
    "Pre-fills meeting titles and attendees on Stop, and back-fills the calendar event's notes with the summary.";
const NOTIFICATIONS_RATIONALE: &str =
    "Used only for 'recording started' / 'summary ready' alerts. Disabled features stay disabled.";

#[tauri::command]
pub fn list_permissions() -> Vec<PermissionRow> {
    #[cfg(target_os = "macos")]
    let (audio_rationale, audio_url) = if attune_core::audio::process_tap::is_supported() {
        (SYSTEM_AUDIO_RATIONALE, SYSTEM_AUDIO_URL)
    } else {
        (SCREEN_RATIONALE, SCREEN_URL)
    };
    #[cfg(not(target_os = "macos"))]
    let (audio_rationale, audio_url) = (SCREEN_RATIONALE, SCREEN_URL);

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
            rationale: audio_rationale.to_string(),
            settings_url: audio_url.to_string(),
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
        Permission::ScreenRecording => {
            #[cfg(target_os = "macos")]
            if attune_core::audio::process_tap::is_supported() {
                SYSTEM_AUDIO_URL
            } else {
                SCREEN_URL
            }
            #[cfg(not(target_os = "macos"))]
            SCREEN_URL
        }
        Permission::Calendar => CALENDAR_URL,
        Permission::Notifications => NOTIFICATIONS_URL,
    };
    open_url(&app, url)
}

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
