use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn remove_entry<'a>(
    entry_name: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), String> {
    state
        .remove_entry(&entry_name)
        .await
        .map_err(|err| err.to_string())
}
