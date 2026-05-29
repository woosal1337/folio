//! Resolve the real macOS icon for an installed app by bundle id and
//! return it as a PNG `data:` URL. Used by the notifications app picker
//! so each app shows its own icon (including Safari / Arc / FaceTime,
//! which brand-icon libraries omit) — and we never embed any third-party
//! logo in our own assets; we read whatever the user has installed.

/// Return `data:image/png;base64,…` for the app with `bundle_id`, or
/// `None` when the app isn't installed or has no resolvable icon.
#[tauri::command]
pub async fn app_icon(bundle_id: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        fetch_icon_png(&bundle_id).map(|png| {
            use base64::Engine as _;
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(png)
            )
        })
    })
    .await
    .ok()
    .flatten()
}

#[cfg(target_os = "macos")]
#[allow(deprecated)] // cocoa 0.26 NSString/NSSize helpers; objc2 migration is out of scope here.
fn fetch_icon_png(bundle_id: &str) -> Option<Vec<u8>> {
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSSize, NSString};
    use objc::{class, msg_send, sel, sel_impl};

    // NSBitmapImageFileTypePNG.
    const PNG_FILE_TYPE: u64 = 4;
    // Icon size we render into the chip (points; @2x handled by the PNG).
    const ICON_PX: f64 = 32.0;

    // SAFETY: every selector below is Apple-documented and read-only. We
    // nil-check each pointer, copy the PNG bytes into an owned Vec before
    // draining the autorelease pool, and let no Obj-C object escape it.
    unsafe {
        let pool: id = msg_send![class!(NSAutoreleasePool), new];

        let result = (|| -> Option<Vec<u8>> {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            if workspace == nil {
                return None;
            }
            let bid = NSString::alloc(nil).init_str(bundle_id);
            let url: id = msg_send![workspace, URLForApplicationWithBundleIdentifier: bid];
            if url == nil {
                return None;
            }
            let path: id = msg_send![url, path];
            if path == nil {
                return None;
            }
            let icon: id = msg_send![workspace, iconForFile: path];
            if icon == nil {
                return None;
            }
            let _: () = msg_send![icon, setSize: NSSize::new(ICON_PX, ICON_PX)];

            let tiff: id = msg_send![icon, TIFFRepresentation];
            if tiff == nil {
                return None;
            }
            let rep: id = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
            if rep == nil {
                return None;
            }
            let props: id = msg_send![class!(NSDictionary), dictionary];
            let png: id = msg_send![rep, representationUsingType: PNG_FILE_TYPE properties: props];
            if png == nil {
                return None;
            }
            let len: usize = msg_send![png, length];
            let bytes: *const u8 = msg_send![png, bytes];
            if bytes.is_null() || len == 0 {
                return None;
            }
            Some(std::slice::from_raw_parts(bytes, len).to_vec())
        })();

        let _: () = msg_send![pool, drain];
        result
    }
}

#[cfg(not(target_os = "macos"))]
fn fetch_icon_png(_bundle_id: &str) -> Option<Vec<u8>> {
    None
}
