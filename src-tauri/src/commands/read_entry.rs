use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn read_entry<'a>(
    entry_name: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<String, String> {
    state
        .get_pw(&entry_name)
        .await
        .map_err(|err| err.to_string())
}
