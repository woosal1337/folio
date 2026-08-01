#[cfg(target_os = "macos")]
pub use imp::{
    authorization_status, reminders_authorization_status, request_access, request_reminders_access,
};

#[cfg(not(target_os = "macos"))]
pub use stub::{
    authorization_status, reminders_authorization_status, request_access, request_reminders_access,
};

pub const STATUS_NOT_DETERMINED: &str = "not_determined";
pub const STATUS_RESTRICTED: &str = "restricted";
pub const STATUS_DENIED: &str = "denied";
pub const STATUS_AUTHORIZED: &str = "authorized";

#[cfg(not(target_os = "macos"))]
mod stub {
    pub fn authorization_status() -> &'static str {
        super::STATUS_NOT_DETERMINED
    }

    pub fn reminders_authorization_status() -> &'static str {
        super::STATUS_NOT_DETERMINED
    }

    pub fn request_access() {}

    pub fn request_reminders_access() {}
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
mod imp {

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
}
