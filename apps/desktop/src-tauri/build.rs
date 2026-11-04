//! Generate Tauri metadata and explicit application-command permissions.

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["get_app_info"])),
    )
    .expect("Failed to build the Tauri application and command permissions");
}
