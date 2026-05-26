//! NSVisualEffectView vibrancy hook. v2 finding 011 / GET-45.
//!
//! Replaces the opaque sidebar / job-strip / popover surfaces with
//! real macOS `NSVisualEffectMaterial` blur. Removes the 'tidy
//! dashboard' web-app smell — the chrome reads as a real Mac app,
//! not a Tauri-shaped iframe.
//!
//! Strategy: at window-creation we adopt a transparent titlebar,
//! disable the default opaque background, and attach an
//! `NSVisualEffectView` to the window's content view sized to the
//! sidebar region. The React side renders directly on top with
//! `background: transparent` on the sidebar so the underlying
//! vibrancy bleeds through.
//!
//! Apple enum constants we use as raw integers (stable since 10.10):
//!   * NSVisualEffectMaterialSidebar = 7
//!   * NSVisualEffectBlendingModeBehindWindow = 0
//!   * NSVisualEffectStateFollowsWindowActiveState = 0
//!   * NSViewWidthSizable | NSViewHeightSizable = 2 | 16 = 18
//!
//! macOS only. Non-macOS targets noop — the existing sidebar uses
//! a normal Tailwind colour.

#[cfg(target_os = "macos")]
const NS_VE_MATERIAL_SIDEBAR: i64 = 7;
#[cfg(target_os = "macos")]
const NS_VE_BLENDING_BEHIND_WINDOW: i64 = 0;
#[cfg(target_os = "macos")]
const NS_VE_STATE_FOLLOWS_WINDOW: i64 = 0;
#[cfg(target_os = "macos")]
const NS_VIEW_AUTORESIZE_FILL: u64 = 2 | 16;

#[cfg(target_os = "macos")]
pub fn install_window_vibrancy(window: &tauri::WebviewWindow) {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSRect;
    use objc::{class, msg_send, sel, sel_impl};

    let Ok(ns_window) = window.ns_window() else {
        tracing::warn!("vibrancy: ns_window() unavailable");
        return;
    };
    let ns_window = ns_window as id;
    // SAFETY: ns_window is a valid NSWindow pointer obtained from Tauri's
    // window handle (the Ok branch above guarantees it). Every Objective-C
    // message below uses Apple-documented selectors and types; the
    // NSVisualEffectView outlives the window because we add it as a
    // subview of contentView, so the window owns it. Each return is
    // checked for nil before further dereferences.
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
        let _: () = msg_send![content_view, addSubview: effect_view positioned: 0u64 relativeTo: nil];
        tracing::info!("vibrancy installed on window {}", window.label());
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_window_vibrancy(_window: &tauri::WebviewWindow) {}
