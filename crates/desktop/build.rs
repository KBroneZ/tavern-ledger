include!("app_commands.rs");

fn main() {
    // Windows looks for the DLLs the app imports by name in System32 only,
    // never first in the app's own folder (LOAD_LIBRARY_SEARCH_SYSTEM32).
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
    println!("cargo:rerun-if-changed=app_commands.rs");
    let manifest = tauri_build::AppManifest::new().commands(APP_COMMANDS);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("the Tauri build step failed");
}
