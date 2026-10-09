// Overlay spike (T-005): a transparent, always-on-top window that lets every
// click through to the window below. Only draws on top; never touches the game.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let overlay = app
                .get_webview_window("overlay")
                .expect("window 'overlay' is defined in tauri.conf.json");
            overlay.set_ignore_cursor_events(true)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run the overlay spike");
}
