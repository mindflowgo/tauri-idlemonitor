const COMMANDS: &[&str] = &["start", "stop", "get_idle_time"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .global_api_script_path("./api-iife.js")
        .build();
}
