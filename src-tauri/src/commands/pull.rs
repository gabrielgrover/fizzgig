use crate::llw_handler::LocalLedgerWorkerHandler;
use land_strider_sdk::LandStrider;
use serde_json::Value;
// use tokio::fs::File;
// use utility::generate_id;
// use walkdir::WalkDir;

#[tauri::command]
pub async fn pull<'a>(
    temp_pw: String,
    pin: String,
    _llw: tauri::State<'a, LocalLedgerWorkerHandler>,
    land_strider: tauri::State<'a, LandStrider>,
) -> Result<Vec<Value>, String> {
    let values = land_strider
        .pull(&pin, &temp_pw)
        .await
        .map_err(|e| e.to_string())?;

    println!("VALUES: {:?}", values);

    Ok(values)

    // let source_dir = llw.get_ledger_dir().await.map_err(|err| err.to_string())?;

    // let mut form = reqwest::multipart::Form::new();

    // for entry in WalkDir::new(source_dir) {
    //     let entry = entry.map_err(|e| e.to_string())?;
    //     let path = entry.path();

    //     if entry.file_type().is_file() {
    //         let file = File::open(path).await.map_err(|e| e.to_string())?;
    //         let part = reqwest::multipart::Part::stream(file);

    //         form = form.part(generate_id(), part);
    //     }
    // }

    // let pw_part = reqwest::multipart::Part::text(temp_pw);

    // form = form.part("pw", pw_part);

    // let push_resp = land_strider.push(form).await.map_err(|e| e.to_string())?;

    // Ok(push_resp)
}
