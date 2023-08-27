use crate::llw_handler::LocalLedgerWorkerHandler;
use land_strider_sdk::{LandStrider, PushResponse};
use tokio::fs::File;
use utility::generate_id;
use walkdir::WalkDir;

#[tauri::command]
pub async fn push<'a>(
    temp_pw: String,
    llw: tauri::State<'a, LocalLedgerWorkerHandler>,
    land_strider: tauri::State<'a, LandStrider>,
) -> Result<PushResponse, String> {
    let source_dir = llw.get_ledger_dir().await.map_err(|err| err.to_string())?;

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

    let push_resp = land_strider.push(form).await.map_err(|e| e.to_string())?;

    Ok(push_resp)
}
