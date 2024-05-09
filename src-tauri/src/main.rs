#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_state;
mod commands;
mod llw_handler;
mod local_ledger_worker;
mod password_ledger_handler;

use app_state::*;
use commands::*;
use land_strider_sdk::*;
use password_ledger_handler::*;
use tokio::sync::Mutex;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

#[tokio::main]
async fn main() {
    let filter = EnvFilter::new("info");
    let s = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer());
    tracing::subscriber::set_global_default(s).expect("setting default tracing subscriber failed");

    let land_strider_config = LandStriderConfig::new("localhost", 3001);
    let land_strider = LandStrider::new(land_strider_config);
    let pw_ledger = Mutex::new(PasswordLedgerHandler::new());

    let app_state = AppState {
        land_strider,
        pw_ledger,
    };

    tauri::Builder::default()
        .manage(app_state)
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
            push,
            pull,
            push_s,
            get_conf_pair,
            resolve_conflict
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
