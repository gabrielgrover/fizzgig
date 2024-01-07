use crate::app_state::AppState;

#[tauri::command]
pub async fn read_entry<'a>(
    entry_name: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<String, String> {
    app_state.pw_ledger.lock().await.get_pw(&entry_name)
}
