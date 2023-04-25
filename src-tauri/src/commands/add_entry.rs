use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn add_entry<'a>(
    entry_name: String,
    val: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), String> {
    state
        .add_entry(&entry_name, &val)
        .await
        .map_err(|err| err.to_string())
}
