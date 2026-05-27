//! macOS native share sheet via `NSSharingServicePicker`.
//!
//! v2 finding 010 / GET-34. The share sheet IS the export menu on
//! macOS — AirDrop, Messages, Mail, Notes, Reminders, third-party
//! share extensions all come for free. The picker is anchored to the
//! current key window's content view.

#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn share_paths(paths: &[std::path::PathBuf]) -> Result<(), String> {
    use cocoa::appkit::NSApp;
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};
    use objc::{class, msg_send, sel, sel_impl};

    if paths.is_empty() {
        return Err("share_paths: no paths".into());
    }
    for p in paths {
        if !p.exists() {
            return Err(format!("share_paths: missing file {}", p.display()));
        }
    }

    // SAFETY: every Objective-C call uses Apple-documented selectors
    // and types. NSString::alloc().init_str + NSURL fileURLWithPath:
    // are infallible on a valid UTF-8 string (we pass `to_string_lossy`).
    // NSSharingServicePicker takes ownership of the items array via
    // ARC-compatible retain semantics, so the temporary id slice can
    // safely be dropped at the end of this block. The picker is
    // anchored to the key window's contentView, which the runtime
    // keeps alive for the lifetime of the app.
    unsafe {
        let mut urls: Vec<id> = Vec::with_capacity(paths.len());
        for p in paths {
            let s = p.to_string_lossy();
            let ns_str: id = NSString::alloc(nil).init_str(s.as_ref());
            let url: id = msg_send![class!(NSURL), fileURLWithPath: ns_str];
            if url == nil {
                return Err(format!("share_paths: NSURL nil for {}", s));
            }
            urls.push(url);
        }
        let items: id = NSArray::arrayWithObjects(nil, &urls);

        // [[NSSharingServicePicker alloc] initWithItems:items]
        let alloc_picker: id = msg_send![class!(NSSharingServicePicker), alloc];
        let picker: id = msg_send![alloc_picker, initWithItems: items];
        if picker == nil {
            return Err("share_paths: NSSharingServicePicker init failed".into());
        }

        // Anchor: centre of the key window's content view, or the
        // screen origin if there's no key window (headless launch).
        let app: id = NSApp();
        let key_window: id = msg_send![app, keyWindow];
        let (anchor_view, anchor_rect): (id, NSRect) = if key_window != nil {
            let view: id = msg_send![key_window, contentView];
            let bounds: NSRect = msg_send![view, bounds];
            let centre = NSRect::new(
                NSPoint::new(
                    bounds.origin.x + bounds.size.width / 2.0,
                    bounds.origin.y + bounds.size.height / 2.0,
                ),
                NSSize::new(1.0, 1.0),
            );
            (view, centre)
        } else {
            (
                nil,
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)),
            )
        };

        if anchor_view == nil {
            return Err("share_paths: no key window — cannot anchor picker".into());
        }

        // NSRectEdgeMinY = 1 — show below the anchor.
        let _: () = msg_send![
            picker,
            showRelativeToRect: anchor_rect
            ofView: anchor_view
            preferredEdge: 1u64
        ];
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn share_paths(_paths: &[std::path::PathBuf]) -> Result<(), String> {
    Err("share_paths: only supported on macOS".into())
}
