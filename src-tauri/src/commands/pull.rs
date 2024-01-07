use crate::app_state::AppState;

#[tauri::command]
pub async fn pull<'a>(
    temp_pw: String,
    pin: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<(), String> {
    let ps = app_state
        .land_strider
        .pull_s(&pin, &temp_pw)
        .await
        .map_err(|e| e.to_string())?;

    let mut pw_ledger = app_state.pw_ledger.lock().await;
    pw_ledger.merge(ps).await?;

    Ok(())
}
