//! Apple EventKit reader (GET-161).
//!
//! Reads the user's Calendar database directly via EventKit — no OAuth,
//! no account creation; Apple's Calendar already syncs whatever providers
//! the user has configured. The pure selection helpers
//! (`next_event`, `current_event`, `detect_conference_url`,
//! `CalendarEvent`) live in `attune_core::calendar`; this module is the
//! macOS-gated Objective-C bridge that fills `CalendarEvent`s from
//! `EKEventStore`.
//!
//! Non-macOS builds get stubs so the workspace still compiles for
//! `cargo check` on Linux CI.

#[cfg(target_os = "macos")]
pub use imp::{authorization_status, read_events};

#[cfg(not(target_os = "macos"))]
pub use stub::{authorization_status, read_events};

/// Authorization states the UI cares about. Mirrors `EKAuthorizationStatus`
/// collapsed to read-relevant buckets.
pub const STATUS_NOT_DETERMINED: &str = "not_determined";
pub const STATUS_RESTRICTED: &str = "restricted";
pub const STATUS_DENIED: &str = "denied";
pub const STATUS_AUTHORIZED: &str = "authorized";

#[cfg(not(target_os = "macos"))]
mod stub {
    use attune_core::calendar::CalendarEvent;

    pub fn authorization_status() -> &'static str {
        super::STATUS_NOT_DETERMINED
    }

    pub fn read_events(_window_secs: f64) -> Vec<CalendarEvent> {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)] // cocoa 0.26 marks its surface deprecated in favour of
                     // objc2; the rest of the app's FFI does the same.
mod imp {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    use attune_core::calendar::{detect_conference_url, CalendarEvent};
    use chrono::{DateTime, Utc};
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    // EKEntityType: events == 0.
    const EK_ENTITY_TYPE_EVENT: i64 = 0;
    // EKAuthorizationStatus values.
    const EK_STATUS_RESTRICTED: i64 = 1;
    const EK_STATUS_DENIED: i64 = 2;
    const EK_STATUS_AUTHORIZED: i64 = 3; // also "full access" on macOS 14+.

    /// Current calendar authorization, collapsed to the read-relevant
    /// buckets the Home "Coming up" card switches on.
    pub fn authorization_status() -> &'static str {
        // SAFETY: a single class-method message with a documented selector
        // and a primitive return; no object lifetimes to manage.
        let status: i64 = unsafe {
            msg_send![
                class!(EKEventStore),
                authorizationStatusForEntityType: EK_ENTITY_TYPE_EVENT
            ]
        };
        match status {
            EK_STATUS_RESTRICTED => super::STATUS_RESTRICTED,
            EK_STATUS_DENIED => super::STATUS_DENIED,
            s if s >= EK_STATUS_AUTHORIZED => super::STATUS_AUTHORIZED,
            _ => super::STATUS_NOT_DETERMINED,
        }
    }

    /// Read calendar events starting within `[now, now + window_secs]`.
    /// Returns an empty vec when access isn't authorized or anything is
    /// missing — the caller (and the pure `next_event` helper) treats an
    /// empty slice as "nothing coming up".
    pub fn read_events(window_secs: f64) -> Vec<CalendarEvent> {
        if authorization_status() != super::STATUS_AUTHORIZED {
            return Vec::new();
        }
        let mut out: Vec<CalendarEvent> = Vec::new();
        // SAFETY: every selector below is Apple-documented. We open an
        // autorelease pool, nil-check each pointer before use, and copy
        // every NSString into an owned Rust String before the pool drains,
        // so no Objective-C object outlives the pool.
        unsafe {
            let pool: id = msg_send![class!(NSAutoreleasePool), new];

            let store: id = msg_send![class!(EKEventStore), alloc];
            let store: id = msg_send![store, init];
            if store == nil {
                let _: () = msg_send![pool, drain];
                return out;
            }

            let start: id = msg_send![class!(NSDate), date];
            let end: id = msg_send![start, dateByAddingTimeInterval: window_secs];
            if start == nil || end == nil {
                let _: () = msg_send![pool, drain];
                return out;
            }

            let predicate: id = msg_send![
                store,
                predicateForEventsWithStartDate: start
                endDate: end
                calendars: nil
            ];
            if predicate == nil {
                let _: () = msg_send![pool, drain];
                return out;
            }

            let events: id = msg_send![store, eventsMatchingPredicate: predicate];
            if events != nil {
                let count: usize = msg_send![events, count];
                for i in 0..count {
                    let ev: id = msg_send![events, objectAtIndex: i];
                    if ev == nil {
                        continue;
                    }
                    if let Some(parsed) = parse_event(ev) {
                        out.push(parsed);
                    }
                }
            }

            let _: () = msg_send![pool, drain];
        }
        out
    }

    /// Map one `EKEvent` into our `CalendarEvent`. Returns `None` when the
    /// event lacks the timestamps we need.
    ///
    /// # Safety
    ///
    /// `ev` must be a non-nil pointer to an Objective-C `EKEvent` instance.
    /// Must be called inside an `NSAutoreleasePool` drain so intermediate
    /// ObjC objects returned by `msg_send!` are freed correctly.
    unsafe fn parse_event(ev: id) -> Option<CalendarEvent> {
        let start_date: id = msg_send![ev, startDate];
        let end_date: id = msg_send![ev, endDate];
        if start_date == nil || end_date == nil {
            return None;
        }
        let starts_at = nsdate_to_utc(start_date)?;
        let ends_at = nsdate_to_utc(end_date)?;

        let id_str: id = msg_send![ev, eventIdentifier];
        let id = nsstring_to_string(id_str).unwrap_or_default();
        let title_str: id = msg_send![ev, title];
        let title = nsstring_to_string(title_str).unwrap_or_else(|| "(untitled)".to_string());

        let location_str: id = msg_send![ev, location];
        let location = nsstring_to_string(location_str).filter(|s| !s.trim().is_empty());

        let notes_str: id = msg_send![ev, notes];
        let notes = nsstring_to_string(notes_str).filter(|s| !s.trim().is_empty());

        let attendees = read_attendees(ev);

        // Conference URL: the event's own URL first, then location/notes.
        let url_obj: id = msg_send![ev, URL];
        let event_url = nsurl_to_string(url_obj);
        let conference_url = event_url
            .as_deref()
            .and_then(detect_conference_url)
            .or_else(|| location.as_deref().and_then(detect_conference_url))
            .or_else(|| notes.as_deref().and_then(detect_conference_url))
            .map(|link| link.url);

        Some(CalendarEvent {
            id,
            title,
            location,
            starts_at,
            ends_at,
            attendees,
            conference_url,
            notes,
        })
    }

    /// Collect attendee email addresses from an `EKEvent`'s participants.
    ///
    /// # Safety
    ///
    /// `ev` must be a non-nil pointer to an Objective-C `EKEvent` instance.
    /// Must be called inside an `NSAutoreleasePool` drain.
    unsafe fn read_attendees(ev: id) -> Vec<String> {
        let mut emails = Vec::new();
        let attendees: id = msg_send![ev, attendees];
        if attendees == nil {
            return emails;
        }
        let count: usize = msg_send![attendees, count];
        for i in 0..count {
            let participant: id = msg_send![attendees, objectAtIndex: i];
            if participant == nil {
                continue;
            }
            // EKParticipant.URL is a `mailto:` NSURL for email attendees.
            let url_obj: id = msg_send![participant, URL];
            if let Some(s) = nsurl_to_string(url_obj) {
                let email = s.strip_prefix("mailto:").unwrap_or(&s).trim().to_string();
                if !email.is_empty() {
                    emails.push(email);
                }
            }
        }
        emails
    }

    /// Convert an `NSDate` to a chrono `DateTime<Utc>` via its Unix epoch.
    ///
    /// # Safety
    ///
    /// `date` must be either nil or a non-dangling pointer to an `NSDate`
    /// instance. The function returns `None` for nil; any other invalid
    /// pointer is undefined behaviour.
    unsafe fn nsdate_to_utc(date: id) -> Option<DateTime<Utc>> {
        if date == nil {
            return None;
        }
        let secs: f64 = msg_send![date, timeIntervalSince1970];
        if !secs.is_finite() {
            return None;
        }
        let millis = (secs * 1000.0) as i64;
        DateTime::<Utc>::from_timestamp_millis(millis)
    }

    /// Copy an `NSString` into an owned Rust `String`.
    ///
    /// # Safety
    ///
    /// `s` must be either nil or a non-dangling pointer to an `NSString`
    /// instance. The UTF-8 pointer returned by `UTF8String` is only valid for
    /// the lifetime of `s`; `CStr::from_ptr` copies it into an owned
    /// allocation before `s` is released by the autorelease pool.
    unsafe fn nsstring_to_string(s: id) -> Option<String> {
        if s == nil {
            return None;
        }
        let utf8: *const c_char = msg_send![s, UTF8String];
        if utf8.is_null() {
            return None;
        }
        Some(CStr::from_ptr(utf8).to_string_lossy().into_owned())
    }

    /// Copy an `NSURL`'s absolute string into an owned Rust `String`.
    ///
    /// # Safety
    ///
    /// `url` must be either nil or a non-dangling pointer to an `NSURL`
    /// instance. Must be called inside an `NSAutoreleasePool` drain so
    /// the `absoluteString` NSString is freed after use.
    unsafe fn nsurl_to_string(url: id) -> Option<String> {
        if url == nil {
            return None;
        }
        let abs: id = msg_send![url, absoluteString];
        nsstring_to_string(abs)
    }
}
