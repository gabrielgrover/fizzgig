#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod llw_handler;
mod local_ledger_worker;

use commands::{add_entry, generate_pw, greet, list, open_collection, read_entry};

use llw_handler::LocalLedgerWorkerHandler;

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .manage(LocalLedgerWorkerHandler::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            add_entry,
            list,
            read_entry,
            open_collection,
            generate_pw
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
