//! Apple EventKit calendar awareness. v2 finding 068 / GET-29.
//!
//! The "no-OAuth" calendar story: read the user's Calendar database
//! directly via EventKit, no Google OAuth, no Microsoft Graph, no
//! account creation. Apple's Calendar app already syncs to whatever
//! providers the user has set up (Google, iCloud, Exchange, ...),
//! and EventKit is the canonical local-read API.
//!
//! Surface:
//!   * The menu bar shows "Next: <title> at <HH:MM> — start recording?"
//!     based on `next_event(now, window)`.
//!   * The Record page pre-fills the title, attendees, and Zoom link
//!     from the matched event.
//!   * The post-meeting summary back-fills the event's notes via
//!     EventKit write (a follow-up; this PR ships the read path).
//!
//! Architecture: the pure helpers here are data-only. The actual
//! EventKit FFI binding (Objective-C runtime calls) lands in a
//! macOS-gated `event_kit_ffi.rs` module in the follow-up. This file
//! defines the type the FFI returns and the selection helpers the
//! menu bar consumes.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conference_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Window the menu bar polls: the next event whose start time falls
/// within `now + lookahead`. Defaults to 30 minutes so the surface
/// catches a meeting that's about to start without showing
/// tomorrow's calendar.
pub const DEFAULT_LOOKAHEAD: Duration = Duration::minutes(30);

/// Return the event whose `starts_at` is the next one >= `now` and
/// <= `now + window`. Ignores all-day events (24h+ duration) so the
/// menu bar surface is meeting-only. Returns None when nothing fits.
pub fn next_event(events: &[CalendarEvent], now: DateTime<Utc>, window: Duration) -> Option<CalendarEvent> {
    let cutoff = now + window;
    events
        .iter()
        .filter(|e| e.starts_at >= now && e.starts_at <= cutoff)
        .filter(|e| (e.ends_at - e.starts_at) < Duration::hours(24))
        .min_by_key(|e| e.starts_at)
        .cloned()
}

/// True when the event is happening right now: `starts_at <= now < ends_at`.
/// Used by the Record page to surface "you're in <event> — start
/// recording?" the instant the user opens the app.
pub fn current_event(events: &[CalendarEvent], now: DateTime<Utc>) -> Option<CalendarEvent> {
    events
        .iter()
        .find(|e| e.starts_at <= now && e.ends_at > now)
        .cloned()
}

/// Best-effort extraction of a conference URL from an event's
/// `location` or `notes`. Walks a small list of providers and picks
/// the first match. Provider detection is intentionally tolerant —
/// users paste these URLs in inconsistent shapes.
pub fn detect_conference_url(text: &str) -> Option<ConferenceLink> {
    for provider in CONFERENCE_PROVIDERS {
        for prefix in provider.url_prefixes {
            if let Some(start) = text.find(prefix) {
                let tail = &text[start..];
                let end = tail
                    .find(|c: char| c.is_whitespace() || c == ')' || c == '>')
                    .unwrap_or(tail.len());
                return Some(ConferenceLink {
                    provider: provider.name.to_string(),
                    url: tail[..end].to_string(),
                });
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConferenceLink {
    pub provider: String,
    pub url: String,
}

struct ProviderSig {
    name: &'static str,
    url_prefixes: &'static [&'static str],
}

const CONFERENCE_PROVIDERS: &[ProviderSig] = &[
    ProviderSig {
        name: "Zoom",
        url_prefixes: &["https://zoom.us/j/", "https://us02web.zoom.us/", "https://us05web.zoom.us/"],
    },
    ProviderSig {
        name: "Google Meet",
        url_prefixes: &["https://meet.google.com/"],
    },
    ProviderSig {
        name: "Microsoft Teams",
        url_prefixes: &["https://teams.microsoft.com/l/meetup-join/"],
    },
    ProviderSig {
        name: "Webex",
        url_prefixes: &["https://webex.com/", "https://www.webex.com/"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ev(id: &str, title: &str, start: DateTime<Utc>, dur_mins: i64) -> CalendarEvent {
        CalendarEvent {
            id: id.into(),
            title: title.into(),
            location: None,
            starts_at: start,
            ends_at: start + Duration::minutes(dur_mins),
            attendees: vec![],
            conference_url: None,
            notes: None,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 26, 9, 0, 0).unwrap()
    }

    #[test]
    fn next_event_picks_the_soonest_within_window() {
        let events = vec![
            ev("a", "Far", now() + Duration::hours(6), 30),
            ev("b", "Soon", now() + Duration::minutes(10), 30),
            ev("c", "Later", now() + Duration::minutes(25), 30),
        ];
        let hit = next_event(&events, now(), DEFAULT_LOOKAHEAD).unwrap();
        assert_eq!(hit.id, "b");
    }

    #[test]
    fn next_event_returns_none_when_nothing_in_window() {
        let events = vec![ev("a", "Tomorrow", now() + Duration::hours(20), 30)];
        assert!(next_event(&events, now(), DEFAULT_LOOKAHEAD).is_none());
    }

    #[test]
    fn next_event_skips_all_day_events() {
        let events = vec![ev("all-day", "Holiday", now() + Duration::minutes(5), 24 * 60)];
        assert!(next_event(&events, now(), DEFAULT_LOOKAHEAD).is_none());
    }

    #[test]
    fn current_event_finds_in_progress() {
        let events = vec![
            ev("past", "Earlier", now() - Duration::hours(2), 30),
            ev("now", "Live", now() - Duration::minutes(5), 30),
            ev("future", "Later", now() + Duration::hours(1), 30),
        ];
        assert_eq!(current_event(&events, now()).unwrap().id, "now");
    }

    #[test]
    fn current_event_none_between_meetings() {
        let events = vec![
            ev("past", "Earlier", now() - Duration::hours(2), 30),
            ev("future", "Later", now() + Duration::hours(1), 30),
        ];
        assert!(current_event(&events, now()).is_none());
    }

    #[test]
    fn detect_conference_url_finds_zoom_in_notes() {
        let notes = "Join us at https://zoom.us/j/123456789?pwd=abc cheers.";
        let link = detect_conference_url(notes).unwrap();
        assert_eq!(link.provider, "Zoom");
        assert!(link.url.contains("123456789"));
    }

    #[test]
    fn detect_conference_url_finds_google_meet() {
        let location = "https://meet.google.com/abc-defg-hij";
        let link = detect_conference_url(location).unwrap();
        assert_eq!(link.provider, "Google Meet");
    }

    #[test]
    fn detect_conference_url_finds_teams() {
        let notes = "Meeting link: https://teams.microsoft.com/l/meetup-join/19%3a... details";
        let link = detect_conference_url(notes).unwrap();
        assert_eq!(link.provider, "Microsoft Teams");
    }

    #[test]
    fn detect_conference_url_returns_none_when_absent() {
        assert!(detect_conference_url("Just a plain note.").is_none());
    }
}
