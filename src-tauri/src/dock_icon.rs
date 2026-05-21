//! macOS Dock icon helper.
//!
//! When the Tauri app is launched from a `.app` bundle, macOS reads the
//! Dock icon from the bundle's `Info.plist`. In dev mode (`tauri dev` /
//! `cargo run`) we run a raw Mach-O binary with no bundle, and macOS
//! hands out a blank Dock icon.
//!
//! This module loads our PNG icon at compile time and assigns it to
//! `NSApplication.applicationIconImage` so the Dock shows the real icon
//! in development too.

// Embed the 1024 master so macOS has plenty of pixel data to downscale
// at any Dock size. Smaller derived rasters can become stale or be
// downscaled poorly by NSImage's interpolation.
#[cfg(target_os = "macos")]
const ICON_PNG: &[u8] = include_bytes!("../icons/logo-source.png");

#[cfg(target_os = "macos")]
pub fn set_dock_icon() {
    use cocoa::appkit::NSApp;
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSUInteger;
    use objc::{class, msg_send, sel, sel_impl};

    // Hash + size logging so the running binary can prove which bytes
    // it's about to assign — invaluable when the Dock is caching old
    // versions and you want to confirm the new one really shipped.
    let len = ICON_PNG.len();
    let prefix = ICON_PNG
        .iter()
        .take(64)
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64));
    tracing::info!(
        bytes = len,
        prefix_hash = prefix,
        "set_dock_icon: loading icon"
    );

    unsafe {
        // NSData *data = [NSData dataWithBytes:bytes length:len];
        let ns_data: id = msg_send![
            class!(NSData),
            dataWithBytes: ICON_PNG.as_ptr() as *const std::ffi::c_void
            length: ICON_PNG.len() as NSUInteger
        ];
        if ns_data == nil {
            tracing::warn!("set_dock_icon: NSData::dataWithBytes returned nil");
            return;
        }

        // NSImage *img = [[NSImage alloc] initWithData:data];
        let alloc_img: id = msg_send![class!(NSImage), alloc];
        let ns_image: id = msg_send![alloc_img, initWithData: ns_data];
        if ns_image == nil {
            tracing::warn!("set_dock_icon: NSImage init failed");
            return;
        }

        // [[NSApplication sharedApplication] setApplicationIconImage:img];
        let app: id = NSApp();
        let _: () = msg_send![app, setApplicationIconImage: ns_image];
        tracing::info!("dock icon set");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_dock_icon() {}
