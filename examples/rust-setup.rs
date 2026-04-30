// Rust-side example: Registering the plugin in your Tauri app.
//
// File: src-tauri/src/lib.rs

fn main() {
    tauri::Builder::default()
        // Option 1: Default (300s / 5 min idle threshold)
        .plugin(tauri_plugin_powermonitor::init())

        // Option 2: Custom threshold
        // .plugin(
        //     tauri_plugin_powermonitor::Builder::new()
        //         .idle_threshold_secs(600) // 10 minutes
        //         .build()
        // )

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
