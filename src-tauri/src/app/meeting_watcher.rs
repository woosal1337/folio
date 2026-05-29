//! Meeting auto-detection watcher. GET-143.
//!
//! Polls `NSWorkspace.runningApplications` on a background thread and
//! emits a `meeting-detected` signal the first time a known
//! conferencing app appears (edge transition off → on). No Accessibility
//! API and no audio inspection — the honest v1 heuristic is "a
//! conferencing app just launched", which covers the native Zoom / Teams
//! / Meet / Webex / Discord / FaceTime clients that spawn when you join
//! a call. Browsers are almost always already running, so they sit in
//! the seed set and never fire (we can't see a tab-level call without
//! the Accessibility API anyway).
//!
//! On detection the watcher stores a [`DetectedMeeting`] on
//! [`AppState`], opens the compact always-on-top HUD window, and emits
//! `meeting-detected` so an already-open HUD can refresh. The HUD reads
//! the pending meeting on mount via the `get_pending_meeting` command.
//!
//! Honours `notify_auto_detected_meetings`, `notification_muted_apps`,
//! and `privacy_mode`: when any of those say "stay quiet" we keep the
//! running-set bookkeeping current (so re-enabling does not replay a
//! backlog) but never surface the HUD.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::app::AppState;

/// Tauri window label for the meeting-detection HUD.
pub const MEETING_HUD_LABEL: &str = "meeting-hud";
/// Tauri event name carrying a [`DetectedMeeting`] payload.
pub const MEETING_DETECTED_EVENT: &str = "meeting-detected";

/// Poll cadence. ~2s keeps the "HUD within ~2s of joining" acceptance
/// target while costing a single cheap NSArray scan.
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Per-app re-fire cooldown. Once we surface a HUD for an app we stay
/// quiet for this long even if it flaps in and out of the running set.
const REFIRE_COOLDOWN: Duration = Duration::from_secs(120);

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

/// Spawn the background detection loop. Called once from the Tauri
/// `setup` hook. No-op on non-macOS targets.
#[cfg(target_os = "macos")]
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    use std::collections::HashMap;

    std::thread::Builder::new()
        .name("meeting-watcher".into())
        .spawn(move || {
            // Bundle ids of monitored apps seen on the previous tick.
            let mut prev_running: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut last_fired: HashMap<String, Instant> = HashMap::new();
            let mut seeded = false;
            // Bounds the main window held before we docked it aside for a
            // meeting (move_aside_in_meetings). `Some` only while we've
            // actively moved the window, so we restore exactly once.
            let mut aside_bounds: Option<crate::app::window_aside::SavedBounds> = None;

            loop {
                let running = running_monitored_bundle_ids();

                // First tick only seeds the baseline so apps already
                // open at launch never fire a (stale) HUD.
                if !seeded {
                    prev_running = running;
                    seeded = true;
                    std::thread::sleep(POLL_INTERVAL);
                    continue;
                }

                let newly_appeared: Vec<String> =
                    running.difference(&prev_running).cloned().collect();
                let any_running = !running.is_empty();
                let just_appeared = !newly_appeared.is_empty();
                prev_running = running;

                // Move-aside: independent of the notify/HUD setting — the
                // user can want their window out of the way without the
                // detection toast. Dock aside on the same off→on edge the
                // HUD fires on; restore once every monitored app is gone
                // (or the setting gets turned off mid-meeting).
                {
                    let state = app.state::<AppState>();
                    let (move_enabled, onboarded) = {
                        let s = state.settings.lock();
                        (s.move_aside_in_meetings, s.onboarding_completed)
                    };
                    if move_enabled && onboarded && just_appeared && aside_bounds.is_none() {
                        aside_bounds = crate::app::window_aside::move_aside(&app);
                    } else if aside_bounds.is_some() && (!any_running || !move_enabled) {
                        if let Some(bounds) = aside_bounds.take() {
                            crate::app::window_aside::restore(&app, bounds);
                        }
                    }
                }

                if !newly_appeared.is_empty() {
                    let state = app.state::<AppState>();
                    let (enabled, muted, privacy, onboarded) = {
                        let s = state.settings.lock();
                        (
                            s.notify_auto_detected_meetings,
                            s.notification_muted_apps.clone(),
                            s.privacy_mode,
                            s.onboarding_completed,
                        )
                    };

                    if enabled && !privacy && onboarded {
                        for bundle_id in newly_appeared {
                            if muted.iter().any(|m| m == &bundle_id) {
                                continue;
                            }
                            if let Some(last) = last_fired.get(&bundle_id) {
                                if last.elapsed() < REFIRE_COOLDOWN {
                                    continue;
                                }
                            }
                            let Some(label) = label_for(&bundle_id) else {
                                continue;
                            };
                            last_fired.insert(bundle_id.clone(), Instant::now());
                            surface_meeting(&app, &state, &bundle_id, label);
                        }
                    }
                }

                std::thread::sleep(POLL_INTERVAL);
            }
        })
        .expect("spawn meeting-watcher thread");
}

#[cfg(not(target_os = "macos"))]
pub fn spawn<R: Runtime>(_app: AppHandle<R>) {}

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

/// Create (or focus) the compact, frameless, always-on-top HUD window in
/// the top-right corner. Never steals focus.
pub fn show_meeting_hud<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    const HUD_W: f64 = 320.0;
    const HUD_H: f64 = 96.0;
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
fn running_monitored_bundle_ids() -> std::collections::HashSet<String> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    use std::collections::HashSet;
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
