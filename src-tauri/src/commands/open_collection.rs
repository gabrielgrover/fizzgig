use crate::app_state::AppState;

#[tauri::command]
pub async fn open_collection<'a>(
    master_pw: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<(), String> {
    let mut ledger = app_state.pw_ledger.lock().await;

    ledger.start(&master_pw)
}
