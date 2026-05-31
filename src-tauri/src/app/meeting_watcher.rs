//! Meeting auto-detection watcher. GET-143.
//!
//! Primary signal (macOS 14.2+): the audio HAL's per-process
//! `IsRunningInput` property — the same one that lights the orange
//! microphone dot in the menu bar. The watcher emits
//! `meeting-detected` on the false → true edge for any monitored
//! conferencing bundle, so the HUD pops the moment you join a Discord
//! voice channel, a Zoom call, a Teams meeting, or a FaceTime — even
//! if the app was already running. This matches Granola's behaviour
//! and replaces the v1 "process launched" heuristic, which only fired
//! on cold launch and missed every dock-resident meeting app.
//!
//! Fallback (older OS / transient HAL unavailability): the original
//! `NSWorkspace.runningApplications` poll. We seed the running-app
//! baseline so apps already open never replay a stale HUD, then
//! surface on each off → on edge. Granola-class fidelity isn't
//! possible there, but the watcher still works.
//!
//! On detection the watcher stores a [`DetectedMeeting`] on
//! [`AppState`], opens the compact always-on-top HUD window, and emits
//! `meeting-detected` so an already-open HUD can refresh. The HUD
//! reads the pending meeting on mount via the `get_pending_meeting`
//! command.
//!
//! Honours `notify_auto_detected_meetings`, `notification_muted_apps`,
//! and `privacy_mode`: when any of those say "stay quiet" we keep the
//! bookkeeping current (so re-enabling does not replay a backlog) but
//! never surface the HUD.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::app::AppState;

/// Tauri window label for the meeting-detection HUD.
pub const MEETING_HUD_LABEL: &str = "meeting-hud";
/// Tauri event name carrying a [`DetectedMeeting`] payload.
pub const MEETING_DETECTED_EVENT: &str = "meeting-detected";

/// Poll cadence. 1s keeps the "HUD within ~1-2s of joining" target.
/// The HAL read is a single `AudioObjectGetPropertyData` + a per-process
/// pair of reads, all on the user's own thread — cheap.
const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Per-app re-fire cooldown. Once we surface a HUD for an app we stay
/// quiet for this long even if the mic flaps. The cooldown is *cleared*
/// when the bundle goes mic-inactive so re-joining the same call still
/// surfaces a fresh prompt.
const REFIRE_COOLDOWN: Duration = Duration::from_secs(120);
/// Minimum sustained mic-active duration before we treat it as a real
/// meeting. Filters out quick capability probes some apps perform on
/// launch (Discord checks device list, browsers warm WebRTC) without
/// dragging the latency past ~2s after a real join.
const ACTIVE_DEBOUNCE: Duration = Duration::from_secs(2);

/// A conferencing app the watcher knows how to recognise. Mirrors the
/// `MONITORABLE_APPS` list in the Notifications settings UI.
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

/// A meeting the watcher surfaced. Serialised both as the
/// `meeting-detected` event payload and as the `get_pending_meeting`
/// command return. The matching TS interface lives inline in `ipc.ts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedMeeting {
    pub bundle_id: String,
    pub app_label: String,
    /// Wall-clock detection time, epoch milliseconds.
    pub detected_at_ms: i64,
}

/// One round of detection. Returned by [`compute_tick`].
struct Tick {
    /// Bundles currently mic-active (HAL) or running (fallback). Drives
    /// the move-aside "any meeting" check.
    any_active: bool,
    /// Bundles that just crossed the debounce + cooldown gate. Each gets
    /// a HUD surfaced this tick (subject to settings).
    just_started: Vec<String>,
    /// True when this tick was a fallback seed; the caller skips
    /// surfacing entirely to avoid replaying apps that were already up.
    seeded_only: bool,
}

#[derive(Default)]
struct WatcherState {
    /// HAL path: bundles seen mic-active on the previous tick.
    hal_prev_active: HashSet<String>,
    /// HAL path: first time each currently-active bundle went mic-active.
    /// Cleared when the bundle goes mic-inactive.
    hal_active_since: HashMap<String, Instant>,

    /// Fallback path: have we seen at least one running-set tick yet?
    fallback_seeded: bool,
    /// Fallback path: bundles seen running on the previous tick.
    fallback_prev_running: HashSet<String>,

    /// Last time we surfaced a HUD per bundle. Used by both paths.
    last_fired: HashMap<String, Instant>,
    /// Bounds saved when we docked the main window aside for a meeting.
    /// `Some` only while we've actively moved the window.
    aside_bounds: Option<crate::app::window_aside::SavedBounds>,
}

/// Spawn the background detection loop. Called once from the Tauri
/// `setup` hook. No-op on non-macOS targets.
#[cfg(target_os = "macos")]
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    std::thread::Builder::new()
        .name("meeting-watcher".into())
        .spawn(move || {
            let mut state = WatcherState::default();
            loop {
                let tick = compute_tick(&mut state);
                if !tick.seeded_only {
                    handle_tick(&app, tick, &mut state);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        })
        .expect("spawn meeting-watcher thread");
}

#[cfg(not(target_os = "macos"))]
pub fn spawn<R: Runtime>(_app: AppHandle<R>) {}

/// Read the current "active meetings" set from whichever source is
/// available. Prefers the audio HAL per-process input signal; falls
/// back to NSWorkspace running-app diff.
#[cfg(target_os = "macos")]
fn compute_tick(state: &mut WatcherState) -> Tick {
    if let Some(procs) = crate::app::audio_input_watcher::snapshot() {
        // HAL primary path. Discard stale fallback state.
        state.fallback_seeded = false;
        state.fallback_prev_running.clear();

        let mic_active: HashSet<String> = procs
            .into_iter()
            .filter(|p| p.input_active && label_for(&p.bundle_id).is_some())
            .map(|p| p.bundle_id)
            .collect();

        // Track per-bundle "first seen mic-active" so we can debounce.
        for bundle_id in mic_active.difference(&state.hal_prev_active) {
            state
                .hal_active_since
                .insert(bundle_id.clone(), Instant::now());
        }
        // Bundles that just went mic-inactive: drop the timer + clear
        // their cooldown so re-joining the same call fires a fresh HUD.
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
        // Fallback path: NSWorkspace running-app edge. Discard HAL state
        // so a return-to-HAL tick re-seeds cleanly.
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

        // No per-stream timing on the fallback path; cooldown alone.
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
    // Move-aside: dock the main window when *any* monitored bundle is
    // active; restore when none are (or the setting was turned off).
    // Dock on the same edge the HUD fires on so the two motions feel
    // linked, not staggered.
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

/// Record the detection, open the HUD, and emit the refresh event.
fn surface_meeting<R: Runtime>(app: &AppHandle<R>, state: &AppState, bundle_id: &str, label: &str) {
    let meeting = DetectedMeeting {
        bundle_id: bundle_id.to_string(),
        app_label: label.to_string(),
        detected_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    tracing::info!(bundle_id, label, "meeting detected");
    *state.pending_meeting.lock() = Some(meeting.clone());

    // Window creation must happen on the main thread on macOS.
    let app_for_window = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(e) = show_meeting_hud(&app_for_window) {
            tracing::warn!(error = %e, "failed to open meeting HUD");
        }
    });

    // Refresh any already-open HUD. New HUDs read the pending meeting on
    // mount, so a missed event here is harmless.
    let _ = app.emit(MEETING_DETECTED_EVENT, meeting);
}

/// Create (or focus) the compact, frameless, always-on-top HUD window
/// in the top-right corner. Never steals focus. The window is
/// transparent so the React pill paints its own corners on top of
/// nothing — the same trick the recording bar uses to render as a
/// round capsule.
pub fn show_meeting_hud<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    // Pill geometry: short and wide so `rounded-full` reads as a true
    // capsule. Width fits "Meeting detected · Discord" + Take Notes +
    // chevron + X with comfortable padding.
    const HUD_W: f64 = 380.0;
    const HUD_H: f64 = 56.0;
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

    // Park the HUD in the top-right of the monitor it landed on.
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let pos = monitor.position();
        let logical_w = size.width as f64 / scale;
        let x = pos.x as f64 / scale + logical_w - HUD_W - MARGIN;
        let y = pos.y as f64 / scale + MARGIN + 28.0; // clear the menu bar
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[allow(deprecated)] // cocoa 0.26 marks its surface deprecated in favour of
                     // objc2; the dock-icon + vibrancy helpers do the same.
fn running_monitored_bundle_ids() -> HashSet<String> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CStr;

    let mut out = HashSet::new();
    // SAFETY: every message below uses Apple-documented selectors on the
    // shared NSWorkspace and its NSRunningApplication array. We check
    // each pointer against nil before dereferencing and copy every
    // NSString into an owned Rust String inside the autorelease pool, so
    // no Objective-C object outlives the pool drain.
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
