use crate::AppState;

#[tauri::command]
pub async fn get_conf_pair<'a>(
    entry_name: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<ConfPair, String> {
    let mut pw_ledger = app_state.pw_ledger.lock().await;
    let pair = pw_ledger.get_conf_pair(&entry_name)?;

    Ok(ConfPair {
        local_pw: pair.0,
        remote_pw: pair.1,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfPair {
    pub local_pw: String,
    pub remote_pw: String,
}
