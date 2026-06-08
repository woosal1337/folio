#[cfg(target_os = "macos")]
pub use imp::{
    authorization_status, read_events, reminders_authorization_status, request_access,
    request_reminders_access,
};

#[cfg(not(target_os = "macos"))]
pub use stub::{
    authorization_status, read_events, reminders_authorization_status, request_access,
    request_reminders_access,
};

pub const STATUS_NOT_DETERMINED: &str = "not_determined";
pub const STATUS_RESTRICTED: &str = "restricted";
pub const STATUS_DENIED: &str = "denied";
pub const STATUS_AUTHORIZED: &str = "authorized";

#[cfg(not(target_os = "macos"))]
mod stub {
    use folio_core::calendar::CalendarEvent;

    pub fn authorization_status() -> &'static str {
        super::STATUS_NOT_DETERMINED
    }

    pub fn reminders_authorization_status() -> &'static str {
        super::STATUS_NOT_DETERMINED
    }

    pub fn read_events(_window_secs: f64) -> Vec<CalendarEvent> {
        Vec::new()
    }

    pub fn request_access() {}

    pub fn request_reminders_access() {}
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
mod imp {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    use chrono::{DateTime, Utc};
    use cocoa::base::{id, nil};
    use folio_core::calendar::{detect_conference_url, CalendarEvent};
    use objc::{class, msg_send, sel, sel_impl};

    const EK_ENTITY_TYPE_EVENT: i64 = 0;
    const EK_ENTITY_TYPE_REMINDER: i64 = 1;

    const EK_STATUS_RESTRICTED: i64 = 1;
    const EK_STATUS_DENIED: i64 = 2;
    const EK_STATUS_AUTHORIZED: i64 = 3;

    fn status_for(entity: i64) -> &'static str {
        let status: i64 =
            unsafe { msg_send![class!(EKEventStore), authorizationStatusForEntityType: entity] };
        match status {
            EK_STATUS_RESTRICTED => super::STATUS_RESTRICTED,
            EK_STATUS_DENIED => super::STATUS_DENIED,
            s if s >= EK_STATUS_AUTHORIZED => super::STATUS_AUTHORIZED,
            _ => super::STATUS_NOT_DETERMINED,
        }
    }

    pub fn authorization_status() -> &'static str {
        status_for(EK_ENTITY_TYPE_EVENT)
    }

    pub fn reminders_authorization_status() -> &'static str {
        status_for(EK_ENTITY_TYPE_REMINDER)
    }

    unsafe fn main_bundle_has_identifier() -> bool {
        use objc2::runtime::{AnyClass, AnyObject};
        let Some(cls) = AnyClass::get(c"NSBundle") else {
            return false;
        };
        let bundle: *mut AnyObject = objc2::msg_send![cls, mainBundle];
        if bundle.is_null() {
            return false;
        }
        let ident: *mut AnyObject = objc2::msg_send![bundle, bundleIdentifier];
        !ident.is_null()
    }

    fn request_full_access(reminders: bool) {
        use std::panic::AssertUnwindSafe;

        use block2::RcBlock;
        use objc2::rc::Retained;
        use objc2::runtime::{AnyClass, AnyObject, Bool};

        if !unsafe { main_bundle_has_identifier() } {
            return;
        }

        let block = RcBlock::new(|_granted: Bool, _err: *mut AnyObject| {});

        let modern = objc2::exception::catch(AssertUnwindSafe(|| unsafe {
            let Some(cls) = AnyClass::get(c"EKEventStore") else {
                return;
            };
            let store: Retained<AnyObject> = objc2::msg_send![cls, new];
            if reminders {
                let _: () = objc2::msg_send![
                    &*store,
                    requestFullAccessToRemindersWithCompletionHandler: &*block
                ];
            } else {
                let _: () = objc2::msg_send![
                    &*store,
                    requestFullAccessToEventsWithCompletionHandler: &*block
                ];
            }
            core::mem::forget(store);
        }));

        if modern.is_ok() {
            return;
        }

        let legacy = objc2::exception::catch(AssertUnwindSafe(|| unsafe {
            let Some(cls) = AnyClass::get(c"EKEventStore") else {
                return;
            };
            let store: Retained<AnyObject> = objc2::msg_send![cls, new];
            let entity = if reminders {
                EK_ENTITY_TYPE_REMINDER
            } else {
                EK_ENTITY_TYPE_EVENT
            };
            let _: () = objc2::msg_send![
                &*store,
                requestAccessToEntityType: entity,
                completionHandler: &*block
            ];
            core::mem::forget(store);
        }));

        if let Err(exception) = legacy {
            let what = if reminders { "reminders" } else { "calendar" };
            tracing::warn!(
                "EventKit {what} access request failed (modern attempt: {modern:?}): {exception:?}"
            );
        }
    }

    pub fn request_access() {
        request_full_access(false);
    }

    pub fn request_reminders_access() {
        request_full_access(true);
    }

    pub fn read_events(window_secs: f64) -> Vec<CalendarEvent> {
        if authorization_status() != super::STATUS_AUTHORIZED {
            return Vec::new();
        }
        let mut out: Vec<CalendarEvent> = Vec::new();

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

    unsafe fn nsurl_to_string(url: id) -> Option<String> {
        if url == nil {
            return None;
        }
        let abs: id = msg_send![url, absoluteString];
        nsstring_to_string(abs)
    }
}
