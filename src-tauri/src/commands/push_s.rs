use crate::app_state::AppState;
use land_strider_sdk::PushResponse;

#[tauri::command]
pub async fn push_s<'a>(
    temp_pw: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<PushResponse, String> {
    let ledger_dump = app_state.pw_ledger.lock().await.get_doc_dump()?;

    app_state
        .land_strider
        .push_s(ledger_dump, temp_pw)
        .await
        .map_err(|e| {
            tracing::error!("land_strider push failed: {:?}", e);
            e.to_string()
        })
}
