use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn list<'a>(
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<Vec<String>, String> {
    state.list_entries().await.map_err(|err| err.to_string())
}
