use crate::app_state::AppState;

#[tauri::command]
pub async fn list<'a>(app_state: tauri::State<'a, AppState>) -> Result<Vec<String>, String> {
    app_state.pw_ledger.lock().await.list_entries()
}
