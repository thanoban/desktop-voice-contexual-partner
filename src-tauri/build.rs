fn main() {
    // Tauri links resource.lib only into application binaries. Unit-test
    // harnesses also retain dialog imports once AppHandle events are exercised,
    // and must select ComCtl32 v6 before Windows resolves TaskDialogIndirect.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Keep icons/version resources, but embed the same default dependency
        // through the linker for every target. Including it in resource.lib too
        // would create duplicate RT_MANIFEST entries in application binaries.
        tauri_build::try_build(
            tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
        )
        .expect("Tauri build failed");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-common-controls.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    } else {
        tauri_build::build();
    }
}
