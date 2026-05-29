fn main() {
    // The Coming-up calendar (GET-161) reads Apple Calendar via EventKit.
    // AppKit/Foundation are linked transitively by Tauri, but EventKit is
    // not — link it explicitly on macOS so the runtime class lookup
    // (`class!(EKEventStore)`) resolves at load time.
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=EventKit");

    tauri_build::build()
}
