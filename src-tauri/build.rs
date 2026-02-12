fn main() {
    // Embed Windows manifest for admin elevation
    #[cfg(windows)]
    {
        let windows_attrs = tauri_build::WindowsAttributes::new()
            .app_manifest(include_str!("app.manifest"));

        tauri_build::try_build(
            tauri_build::Attributes::new().windows_attributes(windows_attrs),
        )
        .expect("Failed to run tauri-build");
    }

    #[cfg(not(windows))]
    {
        tauri_build::build();
    }
}
