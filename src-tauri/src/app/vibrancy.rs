#[cfg(target_os = "macos")]
const NS_VE_MATERIAL_SIDEBAR: i64 = 7;
#[cfg(target_os = "macos")]
const NS_VE_BLENDING_BEHIND_WINDOW: i64 = 0;
#[cfg(target_os = "macos")]
const NS_VE_STATE_FOLLOWS_WINDOW: i64 = 0;
#[cfg(target_os = "macos")]
const NS_VIEW_AUTORESIZE_FILL: u64 = 2 | 16;
#[cfg(target_os = "macos")]
const NS_WINDOW_BELOW: i64 = -1;

#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn install_window_vibrancy(window: &tauri::WebviewWindow) {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSRect;
    use objc::{class, msg_send, sel, sel_impl};

    let Ok(ns_window) = window.ns_window() else {
        tracing::warn!("vibrancy: ns_window() unavailable");
        return;
    };
    let ns_window = ns_window as id;

    unsafe {
        let content_view: id = msg_send![ns_window, contentView];
        if content_view == nil {
            tracing::warn!("vibrancy: contentView is nil");
            return;
        }
        let frame: NSRect = msg_send![content_view, bounds];
        let alloc: id = msg_send![class!(NSVisualEffectView), alloc];
        let effect_view: id = msg_send![alloc, initWithFrame: frame];
        if effect_view == nil {
            tracing::warn!("vibrancy: NSVisualEffectView init failed");
            return;
        }
        let _: () = msg_send![effect_view, setMaterial: NS_VE_MATERIAL_SIDEBAR];
        let _: () = msg_send![effect_view, setBlendingMode: NS_VE_BLENDING_BEHIND_WINDOW];
        let _: () = msg_send![effect_view, setState: NS_VE_STATE_FOLLOWS_WINDOW];
        let _: () = msg_send![effect_view, setAutoresizingMask: NS_VIEW_AUTORESIZE_FILL];
        let _: () = msg_send![content_view, addSubview: effect_view positioned: NS_WINDOW_BELOW relativeTo: nil];
        tracing::info!("vibrancy installed on window {}", window.label());
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_window_vibrancy(_window: &tauri::WebviewWindow) {}
