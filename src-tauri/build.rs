fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=EventKit");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
    }

    tauri_build::build()
}
