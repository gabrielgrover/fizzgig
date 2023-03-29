use crate::{llw_handler::LocalLedgerWorkerHandler, local_ledger_worker::LocalLedgerWorkerErr};

#[tauri::command]
pub async fn list<'a>(
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<Vec<String>, LocalLedgerWorkerErr> {
    state.list_entries().await
}
