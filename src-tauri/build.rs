fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=EventKit");

    tauri_build::build()
}
