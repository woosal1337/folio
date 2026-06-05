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

pub const DEFAULT_LOOKAHEAD: Duration = Duration::minutes(30);

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

pub fn current_event(events: &[CalendarEvent], now: DateTime<Utc>) -> Option<CalendarEvent> {
    events
        .iter()
        .find(|e| e.starts_at <= now && e.ends_at > now)
        .cloned()
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct AttendeeSuggestion {
    pub email: String,

    pub display_name: String,

    pub meeting_count: u32,
}

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
