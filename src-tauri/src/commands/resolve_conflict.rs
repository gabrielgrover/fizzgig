use crate::AppState;

#[tauri::command]
pub async fn resolve_conflict<'a>(
    entry_name: String,
    keep_original: bool,
    app_state: tauri::State<'a, AppState>,
) -> Result<(), String> {
    app_state
        .pw_ledger
        .lock()
        .await
        .resolve(&entry_name, keep_original)
}
