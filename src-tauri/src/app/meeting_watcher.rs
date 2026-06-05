use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::app::AppState;

pub const MEETING_HUD_LABEL: &str = "meeting-hud";

pub const MEETING_DETECTED_EVENT: &str = "meeting-detected";

const POLL_INTERVAL: Duration = Duration::from_secs(1);

const REFIRE_COOLDOWN: Duration = Duration::from_secs(120);

const ACTIVE_DEBOUNCE: Duration = Duration::from_secs(2);

struct MonitoredApp {
    bundle_id: &'static str,
    label: &'static str,
}

const MONITORED_APPS: &[MonitoredApp] = &[
    MonitoredApp {
        bundle_id: "us.zoom.xos",
        label: "Zoom",
    },
    MonitoredApp {
        bundle_id: "com.microsoft.teams2",
        label: "Microsoft Teams",
    },
    MonitoredApp {
        bundle_id: "com.google.meetings",
        label: "Google Meet",
    },
    MonitoredApp {
        bundle_id: "Cisco-Systems.Spark",
        label: "Webex",
    },
    MonitoredApp {
        bundle_id: "com.tinyspeck.slackmacgap",
        label: "Slack",
    },
    MonitoredApp {
        bundle_id: "com.hnc.Discord",
        label: "Discord",
    },
    MonitoredApp {
        bundle_id: "com.apple.FaceTime",
        label: "FaceTime",
    },
];

fn label_for(bundle_id: &str) -> Option<&'static str> {
    MONITORED_APPS
        .iter()
        .find(|a| a.bundle_id == bundle_id)
        .map(|a| a.label)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedMeeting {
    pub bundle_id: String,
    pub app_label: String,

    pub detected_at_ms: i64,
}

struct Tick {
    any_active: bool,

    just_started: Vec<String>,

    seeded_only: bool,
}

#[derive(Default)]
struct WatcherState {
    hal_prev_active: HashSet<String>,

    hal_active_since: HashMap<String, Instant>,

    fallback_seeded: bool,

    fallback_prev_running: HashSet<String>,

    last_fired: HashMap<String, Instant>,

    aside_bounds: Option<crate::app::window_aside::SavedBounds>,
}

#[cfg(target_os = "macos")]
pub fn spawn<R: Runtime>(app: AppHandle<R>) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = Arc::clone(&stop);
    let handle = std::thread::Builder::new()
        .name("meeting-watcher".into())
        .spawn(move || {
            let mut state = WatcherState::default();
            loop {
                if stop_for_thread.load(Ordering::Relaxed) {
                    break;
                }
                let tick = compute_tick(&mut state);
                if !tick.seeded_only {
                    handle_tick(&app, tick, &mut state);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        })
        .expect("spawn meeting-watcher thread");
    (handle, stop)
}

#[cfg(not(target_os = "macos"))]
pub fn spawn<R: Runtime>(_app: AppHandle<R>) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
    let stop = Arc::new(AtomicBool::new(true));
    let handle = std::thread::Builder::new()
        .name("meeting-watcher-stub".into())
        .spawn(|| {})
        .expect("spawn stub");
    (handle, stop)
}

#[cfg(target_os = "macos")]
fn compute_tick(state: &mut WatcherState) -> Tick {
    if let Some(procs) = crate::app::audio_input_watcher::snapshot() {
        state.fallback_seeded = false;
        state.fallback_prev_running.clear();

        let mic_active: HashSet<String> = procs
            .into_iter()
            .filter(|p| p.input_active && label_for(&p.bundle_id).is_some())
            .map(|p| p.bundle_id)
            .collect();

        for bundle_id in mic_active.difference(&state.hal_prev_active) {
            state
                .hal_active_since
                .insert(bundle_id.clone(), Instant::now());
        }

        for bundle_id in state.hal_prev_active.difference(&mic_active) {
            state.hal_active_since.remove(bundle_id);
            state.last_fired.remove(bundle_id);
        }

        let mut just_started = Vec::new();
        for bundle_id in &mic_active {
            let Some(since) = state.hal_active_since.get(bundle_id) else {
                continue;
            };
            if since.elapsed() < ACTIVE_DEBOUNCE {
                continue;
            }
            if state
                .last_fired
                .get(bundle_id)
                .is_some_and(|t| t.elapsed() < REFIRE_COOLDOWN)
            {
                continue;
            }
            just_started.push(bundle_id.clone());
        }

        let any_active = !mic_active.is_empty();
        state.hal_prev_active = mic_active;

        Tick {
            any_active,
            just_started,
            seeded_only: false,
        }
    } else {
        state.hal_prev_active.clear();
        state.hal_active_since.clear();

        let running = running_monitored_bundle_ids();
        if !state.fallback_seeded {
            state.fallback_prev_running = running;
            state.fallback_seeded = true;
            return Tick {
                any_active: false,
                just_started: Vec::new(),
                seeded_only: true,
            };
        }

        let newly: Vec<String> = running
            .difference(&state.fallback_prev_running)
            .cloned()
            .collect();
        let any_active = !running.is_empty();
        state.fallback_prev_running = running;

        let just_started = newly
            .into_iter()
            .filter(|b| {
                state
                    .last_fired
                    .get(b)
                    .is_none_or(|t| t.elapsed() >= REFIRE_COOLDOWN)
            })
            .collect();

        Tick {
            any_active,
            just_started,
            seeded_only: false,
        }
    }
}

fn handle_tick<R: Runtime>(app: &AppHandle<R>, tick: Tick, state: &mut WatcherState) {
    let app_state = app.state::<AppState>();
    let (move_enabled, onboarded) = {
        let s = app_state.settings.lock();
        (s.move_aside_in_meetings, s.onboarding_completed)
    };
    let dock_edge = !tick.just_started.is_empty();
    if move_enabled && onboarded && dock_edge && state.aside_bounds.is_none() {
        state.aside_bounds = crate::app::window_aside::move_aside(app);
    } else if state.aside_bounds.is_some() && (!tick.any_active || !move_enabled) {
        if let Some(bounds) = state.aside_bounds.take() {
            crate::app::window_aside::restore(app, bounds);
        }
    }

    if tick.just_started.is_empty() {
        return;
    }

    let (enabled, muted, privacy, onboarded) = {
        let s = app_state.settings.lock();
        (
            s.notify_auto_detected_meetings,
            s.notification_muted_apps.clone(),
            s.privacy_mode,
            s.onboarding_completed,
        )
    };

    if !(enabled && !privacy && onboarded) {
        return;
    }

    for bundle_id in tick.just_started {
        if muted.iter().any(|m| m == &bundle_id) {
            continue;
        }
        let Some(label) = label_for(&bundle_id) else {
            continue;
        };
        state.last_fired.insert(bundle_id.clone(), Instant::now());
        surface_meeting(app, &app_state, &bundle_id, label);
    }
}

fn surface_meeting<R: Runtime>(app: &AppHandle<R>, state: &AppState, bundle_id: &str, label: &str) {
    let meeting = DetectedMeeting {
        bundle_id: bundle_id.to_string(),
        app_label: label.to_string(),
        detected_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    tracing::info!(bundle_id, label, "meeting detected");
    *state.pending_meeting.lock() = Some(meeting.clone());

    let app_for_window = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(e) = show_meeting_hud(&app_for_window) {
            tracing::warn!(error = %e, "failed to open meeting HUD");
        }
    });

    let _ = app.emit(MEETING_DETECTED_EVENT, meeting);
}

pub fn show_meeting_hud<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    const HUD_W: f64 = 400.0;
    const HUD_H: f64 = 196.0;
    const MARGIN: f64 = 16.0;

    if let Some(existing) = app.get_webview_window(MEETING_HUD_LABEL) {
        let _ = existing.show();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        MEETING_HUD_LABEL,
        WebviewUrl::App("index.html#/meeting-hud".into()),
    )
    .title("Meeting detected")
    .inner_size(HUD_W, HUD_H)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(true)
    .build()?;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let pos = monitor.position();
        let logical_w = size.width as f64 / scale;
        let x = pos.x as f64 / scale + logical_w - HUD_W - MARGIN;
        let y = pos.y as f64 / scale + MARGIN + 28.0;
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn running_monitored_bundle_ids() -> HashSet<String> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CStr;

    let mut out = HashSet::new();

    unsafe {
        let pool: id = msg_send![class!(NSAutoreleasePool), new];
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil {
            let _: () = msg_send![pool, drain];
            return out;
        }
        let apps: id = msg_send![workspace, runningApplications];
        if apps != nil {
            let count: usize = msg_send![apps, count];
            for i in 0..count {
                let app: id = msg_send![apps, objectAtIndex: i];
                if app == nil {
                    continue;
                }
                let bundle: id = msg_send![app, bundleIdentifier];
                if bundle == nil {
                    continue;
                }
                let utf8: *const std::os::raw::c_char = msg_send![bundle, UTF8String];
                if utf8.is_null() {
                    continue;
                }
                let s = CStr::from_ptr(utf8).to_string_lossy().into_owned();
                if label_for(&s).is_some() {
                    out.insert(s);
                }
            }
        }
        let _: () = msg_send![pool, drain];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_for_known_and_unknown_bundles() {
        assert_eq!(label_for("us.zoom.xos"), Some("Zoom"));
        assert_eq!(label_for("com.hnc.Discord"), Some("Discord"));
        assert_eq!(label_for("com.apple.Safari"), None);
        assert_eq!(label_for("com.unknown.app"), None);
    }

    #[test]
    fn detected_meeting_serialises_to_camel_free_snake_keys() {
        let m = DetectedMeeting {
            bundle_id: "us.zoom.xos".into(),
            app_label: "Zoom".into(),
            detected_at_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"bundle_id\":\"us.zoom.xos\""));
        assert!(json.contains("\"app_label\":\"Zoom\""));
        assert!(json.contains("\"detected_at_ms\":1700000000000"));
    }
}
