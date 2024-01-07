use crate::app_state::AppState;

#[tauri::command]
pub async fn remove_entry<'a>(
    entry_name: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<(), String> {
    app_state.pw_ledger.lock().await.remove_entry(&entry_name)
}
