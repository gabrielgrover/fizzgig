use crate::app_state::AppState;
use land_strider_sdk::PushResponse;
use tokio::fs::File;
use utility::generate_id;
use walkdir::WalkDir;

#[tauri::command]
pub async fn push<'a>(
    temp_pw: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<PushResponse, String> {
    let source_dir = app_state.pw_ledger.lock().await.get_ledger_dir()?;
    let mut form = reqwest::multipart::Form::new();

    for entry in WalkDir::new(source_dir) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if entry.file_type().is_file() {
            let file = File::open(path).await.map_err(|e| e.to_string())?;
            let part = reqwest::multipart::Part::stream(file);

            form = form.part(generate_id(), part);
        }
    }

    let pw_part = reqwest::multipart::Part::text(temp_pw);
    form = form.part("pw", pw_part);
    let push_resp = app_state
        .land_strider
        .push(form)
        .await
        .map_err(|e| e.to_string())?;

    Ok(push_resp)
}
