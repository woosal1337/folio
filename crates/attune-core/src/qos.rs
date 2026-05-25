//! Thread Quality-of-Service tagging.
//!
//! macOS schedulers route work between P-cores and E-cores based on
//! the QoS class of the calling thread. Audio capture threads ask the
//! kernel for `USER_INTERACTIVE` so they stay on P-cores and meet
//! their 10-20ms callback deadlines; batch whisper threads ask for
//! `USER_INITIATED` so the scheduler can land them on E-cores when
//! the user is interacting elsewhere; long-running maintenance jobs
//! ask for `UTILITY` or `BACKGROUND` to stay fully off the P-cores.
//!
//! The wrapper is a no-op on non-macOS targets so the workspace still
//! builds + runs unchanged on Linux CI runners. Calls are cheap — a
//! single libc syscall — and idempotent. The audio-callback caller
//! uses an `Once` guard so it only tags itself on the first sample
//! frame; the spawn_blocking caller invokes it once at the top of the
//! closure. v2 finding 064 / GET-99.

/// Quality-of-service classes that map 1:1 onto the macOS
/// `QOS_CLASS_*` constants. Lower discriminants correspond to higher
/// scheduling priority on Apple Silicon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosClass {
    /// Capture callbacks. Must meet realtime deadlines.
    UserInteractive,
    /// Whisper transcription. Important but the user can wait a few
    /// hundred ms without noticing.
    UserInitiated,
    /// Default — leave the scheduler alone.
    Default,
    /// Background indexing, sidecar generation, large file moves.
    Utility,
    /// Cleanup jobs, sync, anything genuinely throwaway.
    Background,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::QosClass;
    use tracing::warn;

    // The libc crate exposes the constants and the syscall on macOS
    // targets. Casts are spelled out so the wrapper compiles cleanly
    // on every libc release that has touched the type signature.
    fn raw(class: QosClass) -> libc::qos_class_t {
        match class {
            QosClass::UserInteractive => libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE,
            QosClass::UserInitiated => libc::qos_class_t::QOS_CLASS_USER_INITIATED,
            QosClass::Default => libc::qos_class_t::QOS_CLASS_DEFAULT,
            QosClass::Utility => libc::qos_class_t::QOS_CLASS_UTILITY,
            QosClass::Background => libc::qos_class_t::QOS_CLASS_BACKGROUND,
        }
    }

    /// Tag the current pthread with the given QoS class. Returns true
    /// on success. Logs at WARN and returns false if the syscall is
    /// rejected (e.g. on a thread the kernel does not consider
    /// taggable).
    pub fn set_thread_qos(class: QosClass) -> bool {
        // SAFETY: `pthread_set_qos_class_self_np` is documented as
        // safe to call from any thread; the second argument is a
        // relative-priority offset which we pin to 0 (the default
        // for the class).
        let rc = unsafe { libc::pthread_set_qos_class_self_np(raw(class), 0) };
        if rc == 0 {
            true
        } else {
            warn!(rc, ?class, "pthread_set_qos_class_self_np failed");
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::QosClass;

    /// On non-macOS targets QoS classes do not exist. We return true
    /// to keep call sites uniform; the function is otherwise a no-op.
    pub fn set_thread_qos(_class: QosClass) -> bool {
        true
    }
}

pub use imp::set_thread_qos;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_thread_qos_returns_true_on_supported_classes() {
        // On macOS this exercises the real syscall; on other targets
        // the stub returns true unconditionally. Either way the call
        // must not panic.
        assert!(set_thread_qos(QosClass::Default));
        assert!(set_thread_qos(QosClass::Utility));
    }
}
