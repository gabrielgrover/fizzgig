use crate::{llw_handler::LocalLedgerWorkerHandler, local_ledger_worker::LocalLedgerWorkerErr};

#[tauri::command]
pub async fn open_collection<'a>(
    ledger_name: String,
    master_pw: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), LocalLedgerWorkerErr> {
    state.start_worker(&ledger_name, &master_pw).await
}
