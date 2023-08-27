#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod llw_handler;
mod local_ledger_worker;

use commands::{
    add_entry, export_ledger, generate_pw, greet, list, open_collection, push, read_entry,
    regen_pw, remove_entry,
};

use land_strider_sdk::*;
use llw_handler::LocalLedgerWorkerHandler;

#[tokio::main]
async fn main() {
    let land_strider_config = LandStriderConfig::new("localhost", 3001);
    let land_strider = LandStrider::new(land_strider_config);

    tauri::Builder::default()
        .manage(LocalLedgerWorkerHandler::new())
        .manage(land_strider)
        .invoke_handler(tauri::generate_handler![
            greet,
            add_entry,
            list,
            read_entry,
            open_collection,
            generate_pw,
            regen_pw,
            remove_entry,
            export_ledger,
            push
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
