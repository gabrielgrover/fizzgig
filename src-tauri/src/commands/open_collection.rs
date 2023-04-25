use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn open_collection<'a>(
    ledger_name: String,
    master_pw: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), String> {
    state
        .start_worker(&ledger_name, &master_pw)
        .await
        .map_err(|err| err.to_string())
}
