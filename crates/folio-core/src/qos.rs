#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosClass {
    UserInteractive,

    UserInitiated,

    Default,

    Utility,

    Background,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::QosClass;
    use tracing::warn;

    fn raw(class: QosClass) -> libc::qos_class_t {
        match class {
            QosClass::UserInteractive => libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE,
            QosClass::UserInitiated => libc::qos_class_t::QOS_CLASS_USER_INITIATED,
            QosClass::Default => libc::qos_class_t::QOS_CLASS_DEFAULT,
            QosClass::Utility => libc::qos_class_t::QOS_CLASS_UTILITY,
            QosClass::Background => libc::qos_class_t::QOS_CLASS_BACKGROUND,
        }
    }

    pub fn set_thread_qos(class: QosClass) -> bool {
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
        assert!(set_thread_qos(QosClass::Default));
        assert!(set_thread_qos(QosClass::Utility));
    }
}
