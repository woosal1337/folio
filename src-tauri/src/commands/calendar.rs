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

use attune_core::calendar::{derive_attendee_suggestions, AttendeeSuggestion};

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
