//! GET-132 — calendar attendee suggestions IPC.
//!
//! Reads recent events from Apple Calendar via EventKit and derives a
//! deduped, domain-matched, count-filtered list of teammate
//! suggestions using `attune_core::calendar::derive_attendee_suggestions`.
//!
//! v1 ships the IPC surface + the pure derivation helper. The
//! EventKit FFI reader is deferred (see the top-of-file comment in
//! `crates/attune-core/src/calendar.rs`), so for now this command
//! returns an empty `Vec` and the frontend renders the empty-state
//! affordance. When the FFI reader lands, this command flips to
//! reading real events without any frontend change.

use attune_core::calendar::{
    derive_attendee_suggestions, next_event, AttendeeSuggestion, CalendarEvent, DEFAULT_LOOKAHEAD,
};

use crate::app::event_kit;

/// The window the Home "Coming up" card looks ahead (GET-161). Wider than
/// the menu bar's 30-min default so the card can surface the next meeting
/// earlier; `next_event` still picks the soonest within its lookahead.
const COMING_UP_LOOKAHEAD_SECS: f64 = 2.0 * 60.0 * 60.0;

/// Calendar authorization status for the Home empty/permission state
/// (GET-161): "authorized" | "denied" | "restricted" | "not_determined".
#[tauri::command]
pub fn calendar_authorization_status() -> String {
    event_kit::authorization_status().to_string()
}

/// The next upcoming meeting from Apple Calendar (GET-161), or `None`
/// when nothing is coming up soon / access isn't granted. Reads a 2h
/// window via EventKit, then applies the pure `next_event` selection
/// (soonest within the menu-bar lookahead, all-day events skipped).
#[tauri::command]
pub fn next_calendar_event() -> Option<CalendarEvent> {
    let events = event_kit::read_events(COMING_UP_LOOKAHEAD_SECS);
    next_event(&events, chrono::Utc::now(), DEFAULT_LOOKAHEAD)
}

/// List teammate suggestions for the onboarding invite screen.
///
/// * `user_email` — the signed-in user's email (lowercased and
///   excluded from the suggestions).
/// * `domain_filter` — when non-empty, restricts suggestions to
///   attendees with the matching email domain.
/// * `window_days` — accepted for forward-compat; ignored by the
///   v1 stub.
/// * `min_count` — only attendees appearing in at least this many
///   distinct events are returned.
#[tauri::command]
pub fn list_attendee_suggestions(
    user_email: String,
    domain_filter: String,
    window_days: u32,
    min_count: u32,
) -> Vec<AttendeeSuggestion> {
    let _ = window_days;
    // Until the EventKit FFI reader lands, we have no events to feed
    // the helper with. Calling it with an empty slice keeps the wire
    // contract honest and makes the eventual swap a one-line change.
    let events = Vec::new();
    let domain_opt = if domain_filter.trim().is_empty() {
        None
    } else {
        Some(domain_filter.as_str())
    };
    derive_attendee_suggestions(&events, &user_email, domain_opt, min_count)
}
