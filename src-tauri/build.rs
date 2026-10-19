fn main() {
    // El manifiesto (Common Controls v6) se incrusta con el enlazador para todos
    // los binarios, incluidos los de test: sin él, `cargo test` falla en Windows
    // con STATUS_ENTRYPOINT_NOT_FOUND.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::env::current_dir()
            .unwrap()
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rustc-link-arg=/WX");
    }
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("error en tauri-build");
}
