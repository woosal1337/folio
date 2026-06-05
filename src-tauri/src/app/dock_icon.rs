#[cfg(target_os = "macos")]
const ICON_PNG: &[u8] = include_bytes!("../../icons/logo-source.png");

#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn set_dock_icon() {
    use cocoa::appkit::NSApp;
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSUInteger;
    use objc::{class, msg_send, sel, sel_impl};

    let len = ICON_PNG.len();
    let prefix = ICON_PNG
        .iter()
        .take(64)
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64));
    tracing::debug!(
        bytes = len,
        prefix_hash = prefix,
        "set_dock_icon: loading icon"
    );

    unsafe {
        let ns_data: id = msg_send![
            class!(NSData),
            dataWithBytes: ICON_PNG.as_ptr() as *const std::ffi::c_void
            length: ICON_PNG.len() as NSUInteger
        ];
        if ns_data == nil {
            tracing::warn!("set_dock_icon: NSData::dataWithBytes returned nil");
            return;
        }

        let alloc_img: id = msg_send![class!(NSImage), alloc];
        let ns_image: id = msg_send![alloc_img, initWithData: ns_data];
        if ns_image == nil {
            tracing::warn!("set_dock_icon: NSImage init failed");
            return;
        }

        let app: id = NSApp();
        let _: () = msg_send![app, setApplicationIconImage: ns_image];
        tracing::debug!("dock icon set");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_dock_icon() {}
