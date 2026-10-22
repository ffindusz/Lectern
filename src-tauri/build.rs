fn main() {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if !windows {
        tauri_build::build();
        return;
    }
    // tauri-build only embeds its application manifest into *binaries*. The manifest
    // selects Common Controls v6, which the dialog plugin imports from, so without it the
    // unit-test executables fail to start (STATUS_ENTRYPOINT_NOT_FOUND). Embed our own
    // manifest into every target instead, and tell tauri-build not to add a second one.
    println!("cargo:rerun-if-changed=windows/app.manifest");
    println!("cargo:rerun-if-changed=windows/manifest.rc");
    embed_resource::compile_for_everything("windows/manifest.rc", embed_resource::NONE)
        .manifest_required()
        .expect("failed to embed the Windows application manifest");
    let attrs = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    tauri_build::try_build(attrs).expect("tauri-build failed");
}
