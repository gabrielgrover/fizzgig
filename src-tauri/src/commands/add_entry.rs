use crate::app_state::AppState;

#[tauri::command]
pub async fn add_entry<'a>(
    entry_name: String,
    val: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<(), String> {
    app_state
        .pw_ledger
        .lock()
        .await
        .add_entry(&entry_name, &val)
}
