use crate::{llw_handler::LocalLedgerWorkerHandler, local_ledger_worker::LocalLedgerWorkerErr};

#[tauri::command]
pub async fn read_entry<'a>(
    entry_name: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<String, LocalLedgerWorkerErr> {
    state.get_pw(&entry_name).await
}
