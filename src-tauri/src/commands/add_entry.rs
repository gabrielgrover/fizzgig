use crate::{llw_handler::LocalLedgerWorkerHandler, local_ledger_worker::LocalLedgerWorkerErr};

#[tauri::command]
pub async fn add_entry<'a>(
    entry_name: String,
    val: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), LocalLedgerWorkerErr> {
    state.add_entry(&entry_name, &val).await
}
