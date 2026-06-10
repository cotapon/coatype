fn main() {
    println!("cargo:rerun-if-changed=icons/tray-icon.png");
    tauri_build::build()
}
