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
pub fn next_event(
    events: &[CalendarEvent],
    now: DateTime<Utc>,
    window: Duration,
) -> Option<CalendarEvent> {
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

/// GET-132 — a single teammate suggestion derived from recent
/// calendar events. Returned by `derive_attendee_suggestions` and
/// surfaced by the onboarding invite-teammates screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct AttendeeSuggestion {
    /// Email address as it appeared in the calendar attendee list.
    /// Always lowercased.
    pub email: String,
    /// Display name when EventKit knew one. Empty string when the
    /// attendee entry only carried an email (the UI falls back to
    /// the local part of the email in that case).
    pub display_name: String,
    /// Number of distinct events in the input window where this
    /// attendee appeared. Drives the "N meetings together" caption.
    pub meeting_count: u32,
}

/// Derive teammate suggestions from a window of calendar events.
///
/// Pure on-device computation — the input slice already lives on
/// the user's Mac, the output is sorted suggestions. Filters apply
/// in order:
///   1. Drop events whose attendees vector is empty.
///   2. Drop the user's own email (case-insensitive) from each event.
///   3. Optionally filter to attendees whose domain matches
///      `user_domain` (case-insensitive). Passing `None` keeps all
///      domains.
///   4. Group remaining emails across events, count distinct event
///      occurrences, and keep only those meeting `min_count`.
///   5. Sort by meeting count (desc), then email (asc) for stable
///      output.
pub fn derive_attendee_suggestions(
    events: &[CalendarEvent],
    user_email: &str,
    user_domain: Option<&str>,
    min_count: u32,
) -> Vec<AttendeeSuggestion> {
    let user_email_lower = user_email.trim().to_ascii_lowercase();
    let domain_lower = user_domain.map(|d| d.trim().to_ascii_lowercase());

    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for event in events {
        // Each attendee within one event counts at most once toward
        // the meeting tally for that event (dedupe within event).
        let mut seen_in_event: std::collections::HashSet<String> = std::collections::HashSet::new();
        for raw in &event.attendees {
            let email = raw.trim().to_ascii_lowercase();
            if email.is_empty() || email == user_email_lower {
                continue;
            }
            if let Some(domain) = &domain_lower {
                let Some(at) = email.find('@') else { continue };
                let attendee_domain = &email[at + 1..];
                if attendee_domain != domain {
                    continue;
                }
            }
            seen_in_event.insert(email);
        }
        for email in seen_in_event {
            *counts.entry(email).or_insert(0) += 1;
        }
    }

    let mut out: Vec<AttendeeSuggestion> = counts
        .into_iter()
        .filter(|(_, c)| *c >= min_count)
        .map(|(email, meeting_count)| AttendeeSuggestion {
            email,
            display_name: String::new(),
            meeting_count,
        })
        .collect();
    out.sort_by(|a, b| {
        b.meeting_count
            .cmp(&a.meeting_count)
            .then_with(|| a.email.cmp(&b.email))
    });
    out
}

struct ProviderSig {
    name: &'static str,
    url_prefixes: &'static [&'static str],
}

const CONFERENCE_PROVIDERS: &[ProviderSig] = &[
    ProviderSig {
        name: "Zoom",
        url_prefixes: &[
            "https://zoom.us/j/",
            "https://us02web.zoom.us/",
            "https://us05web.zoom.us/",
        ],
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
        let events = vec![ev(
            "all-day",
            "Holiday",
            now() + Duration::minutes(5),
            24 * 60,
        )];
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

    fn ev_with(id: &str, attendees: &[&str]) -> CalendarEvent {
        let mut e = ev(id, "Meeting", now(), 30);
        e.attendees = attendees.iter().map(|s| (*s).to_string()).collect();
        e
    }

    #[test]
    fn derive_attendee_drops_user_and_counts_distinct_events() {
        let events = vec![
            ev_with("1", &["me@acme.com", "alice@acme.com", "bob@acme.com"]),
            ev_with("2", &["ME@ACME.COM", "alice@acme.com"]),
            ev_with("3", &["me@acme.com", "bob@acme.com"]),
        ];
        let out = derive_attendee_suggestions(&events, "me@acme.com", None, 1);
        // Alice in 2 events, Bob in 2 events. Both meet min_count=1.
        // Sort: equal counts → alphabetical by email.
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].email, "alice@acme.com");
        assert_eq!(out[0].meeting_count, 2);
        assert_eq!(out[1].email, "bob@acme.com");
        assert_eq!(out[1].meeting_count, 2);
    }

    #[test]
    fn derive_attendee_filters_by_domain_when_provided() {
        let events = vec![ev_with(
            "1",
            &["alice@acme.com", "carl@vendor.io", "dave@acme.com"],
        )];
        let out = derive_attendee_suggestions(&events, "me@acme.com", Some("acme.com"), 1);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|s| s.email.ends_with("@acme.com")));
    }

    #[test]
    fn derive_attendee_applies_min_count_threshold() {
        let events = vec![
            ev_with("1", &["alice@acme.com", "bob@acme.com"]),
            ev_with("2", &["alice@acme.com"]),
            ev_with("3", &["alice@acme.com"]),
        ];
        let out = derive_attendee_suggestions(&events, "me@acme.com", None, 3);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].email, "alice@acme.com");
        assert_eq!(out[0].meeting_count, 3);
    }

    #[test]
    fn derive_attendee_dedupes_within_one_event() {
        let events = vec![ev_with("1", &["alice@acme.com", "alice@acme.com"])];
        let out = derive_attendee_suggestions(&events, "me@acme.com", None, 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].meeting_count, 1);
    }

    #[test]
    fn derive_attendee_sorts_by_count_desc_then_email_asc() {
        let events = vec![
            ev_with("1", &["zoe@acme.com", "bob@acme.com"]),
            ev_with("2", &["zoe@acme.com", "bob@acme.com"]),
            ev_with("3", &["zoe@acme.com"]),
        ];
        let out = derive_attendee_suggestions(&events, "me@acme.com", None, 1);
        assert_eq!(out[0].email, "zoe@acme.com");
        assert_eq!(out[0].meeting_count, 3);
        assert_eq!(out[1].email, "bob@acme.com");
        assert_eq!(out[1].meeting_count, 2);
    }
}
