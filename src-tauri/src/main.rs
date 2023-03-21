#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
use commands::{add_entry, greet, list, read_entry};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, add_entry, list, read_entry])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
