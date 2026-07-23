use folio_core::calendar::{
    derive_attendee_suggestions, next_event, AttendeeSuggestion, CalendarEvent, DEFAULT_LOOKAHEAD,
};

use crate::app::event_kit;

const COMING_UP_LOOKAHEAD_SECS: f64 = 2.0 * 60.0 * 60.0;

#[tauri::command]
pub fn calendar_authorization_status() -> String {
    event_kit::authorization_status().to_string()
}

#[tauri::command]
pub fn next_calendar_event() -> Option<CalendarEvent> {
    let events = event_kit::read_events(COMING_UP_LOOKAHEAD_SECS);
    next_event(&events, chrono::Utc::now(), DEFAULT_LOOKAHEAD)
}

#[tauri::command]
pub fn list_calendar_events(window_days: u32) -> Vec<CalendarEvent> {
    let window_secs = (window_days.max(1) as f64) * 24.0 * 60.0 * 60.0;
    let mut events = event_kit::read_events(window_secs);
    events.sort_by_key(|e| e.starts_at);
    events
}

#[tauri::command]
pub fn list_attendee_suggestions(
    user_email: String,
    domain_filter: String,
    window_days: u32,
    min_count: u32,
) -> Vec<AttendeeSuggestion> {
    let window_secs = (window_days.max(1) as f64) * 24.0 * 60.0 * 60.0;
    let events = event_kit::read_events(window_secs);
    let domain_opt = if domain_filter.trim().is_empty() {
        None
    } else {
        Some(domain_filter.as_str())
    };
    derive_attendee_suggestions(&events, &user_email, domain_opt, min_count)
}
