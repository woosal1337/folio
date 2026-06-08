use folio_core::permissions::{Permission, PermissionRow, PermissionStatus};

const MIC_URL: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";

const SCREEN_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";

const CALENDAR_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";
const REMINDERS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders";
const NOTIFICATIONS_URL: &str = "x-apple.systempreferences:com.apple.preference.notifications";

const MIC_RATIONALE: &str =
    "We record what you say. Without microphone access, your half of every meeting is silent.";

const SCREEN_RATIONALE: &str =
    "We record what the other side says by capturing system audio. Screen Recording is the macOS API that allows it.";

const REMINDERS_RATIONALE: &str =
    "Syncs extracted action items into your Apple Reminders list, if you turn that on.";
const NOTIFICATIONS_RATIONALE: &str =
    "Used only for 'recording started' / 'summary ready' alerts. Disabled features stay disabled.";

#[tauri::command]
pub fn list_permissions() -> Vec<PermissionRow> {
    let (audio_rationale, audio_url) = (SCREEN_RATIONALE, SCREEN_URL);

    #[cfg(target_os = "macos")]
    let (mic, screen, reminders) = (mac::mic_status(), mac::screen_status(), reminders_status());
    #[cfg(not(target_os = "macos"))]
    let (mic, screen, reminders) = (
        PermissionStatus::Unknown,
        PermissionStatus::Unknown,
        PermissionStatus::Unknown,
    );

    vec![
        PermissionRow {
            permission: Permission::Microphone,
            status: mic,
            rationale: MIC_RATIONALE.to_string(),
            settings_url: MIC_URL.to_string(),
        },
        PermissionRow {
            permission: Permission::ScreenRecording,
            status: screen,
            rationale: audio_rationale.to_string(),
            settings_url: audio_url.to_string(),
        },
        PermissionRow {
            permission: Permission::Reminders,
            status: reminders,
            rationale: REMINDERS_RATIONALE.to_string(),
            settings_url: REMINDERS_URL.to_string(),
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
pub fn request_permission(app: tauri::AppHandle, permission: Permission) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match permission {
            Permission::Microphone => match mac::mic_status() {
                PermissionStatus::Granted => Ok(()),
                PermissionStatus::NotDetermined => {
                    mac::request_mic();
                    Ok(())
                }
                _ => open_permission_settings(app, permission),
            },
            Permission::ScreenRecording => {
                if mac::screen_status() == PermissionStatus::Granted {
                    Ok(())
                } else {
                    mac::request_screen();
                    open_permission_settings(app, permission)
                }
            }
            Permission::Calendar => request_calendar_access(app),
            Permission::Reminders => match reminders_status() {
                PermissionStatus::Granted => Ok(()),
                PermissionStatus::NotDetermined => {
                    crate::app::event_kit::request_reminders_access();
                    Ok(())
                }
                _ => open_permission_settings(app, permission),
            },
            Permission::Notifications => open_permission_settings(app, permission),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (&app, permission);
        Err("requesting permissions is only supported on macOS".into())
    }
}

#[cfg(target_os = "macos")]
fn map_ek_status(s: &str) -> PermissionStatus {
    use crate::app::event_kit;
    match s {
        event_kit::STATUS_AUTHORIZED => PermissionStatus::Granted,
        event_kit::STATUS_DENIED => PermissionStatus::Denied,
        event_kit::STATUS_RESTRICTED => PermissionStatus::Restricted,
        _ => PermissionStatus::NotDetermined,
    }
}

#[cfg(target_os = "macos")]
fn calendar_status() -> PermissionStatus {
    map_ek_status(crate::app::event_kit::authorization_status())
}

#[cfg(target_os = "macos")]
fn reminders_status() -> PermissionStatus {
    map_ek_status(crate::app::event_kit::reminders_authorization_status())
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
mod mac {
    use block::ConcreteBlock;
    use cocoa::base::{id, nil, BOOL};
    use cocoa::foundation::NSString;
    use objc::{class, msg_send, sel, sel_impl};

    use folio_core::permissions::PermissionStatus;

    pub fn mic_status() -> PermissionStatus {
        unsafe {
            let audio: id = NSString::alloc(nil).init_str("soun");
            let status: i64 =
                msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: audio];
            match status {
                3 => PermissionStatus::Granted,
                2 => PermissionStatus::Denied,
                1 => PermissionStatus::Restricted,
                _ => PermissionStatus::NotDetermined,
            }
        }
    }

    pub fn request_mic() {
        unsafe {
            let audio: id = NSString::alloc(nil).init_str("soun");

            let handler = ConcreteBlock::new(|_granted: BOOL| {}).copy();
            let _: () = msg_send![
                class!(AVCaptureDevice),
                requestAccessForMediaType: audio
                completionHandler: &*handler
            ];
        }
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    pub fn screen_status() -> PermissionStatus {
        if unsafe { CGPreflightScreenCaptureAccess() } {
            PermissionStatus::Granted
        } else {
            PermissionStatus::NotDetermined
        }
    }

    pub fn request_screen() {
        unsafe {
            let _ = CGRequestScreenCaptureAccess();
        }
    }
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
        Permission::Reminders => REMINDERS_URL,
        Permission::Notifications => NOTIFICATIONS_URL,
    };
    open_url(&app, url)
}

#[tauri::command]
pub fn request_calendar_access(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match calendar_status() {
            PermissionStatus::Granted => Ok(()),
            PermissionStatus::NotDetermined => {
                crate::app::event_kit::request_access();
                Ok(())
            }
            _ => open_url(&app, CALENDAR_URL),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        open_url(&app, CALENDAR_URL)
    }
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
