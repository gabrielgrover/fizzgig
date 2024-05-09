use crate::app_state::AppState;
use crate::EntryMetaData;

#[tauri::command]
pub async fn list<'a>(app_state: tauri::State<'a, AppState>) -> Result<Vec<EntryMetaData>, String> {
    app_state.pw_ledger.lock().await.list_entry_meta_data()
}
