use crate::llw_handler::LocalLedgerWorkerHandler;
use tokio::fs::File;
use utility::generate_id;
use walkdir::WalkDir;

#[tauri::command]
pub async fn push<'a>(state: tauri::State<'a, LocalLedgerWorkerHandler>) -> Result<(), String> {
    let source_dir = state
        .get_ledger_dir()
        .await
        .map_err(|err| err.to_string())?;

    //let mut files = vec![];

    let mut form = reqwest::multipart::Form::new();

    for entry in WalkDir::new(source_dir) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if entry.file_type().is_file() {
            let file = File::open(path).await.map_err(|e| e.to_string())?;
            //let reader = BufReader::new(file);
            //let s = BufStream::new(file);
            //let stream = reqwest::Body::wrap_stream(s);
            //let stream = reqwest::Body::wrap_stream(reader.lines);
            // let mut buffer = vec![];
            // file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

            //let stream = reqwest::Body::wrap_stream(file);

            //let part = reqwest::multipart::Part::
            let part = reqwest::multipart::Part::stream(file);

            form = form.part(generate_id(), part);

            //parts.push(part);

            //form.part(generate_id(), part);

            //form.part(generate_id(), buffer);
            //files.push((generate_id(), file));
        }
    }

    let client = reqwest::Client::new();

    let resp = client
        .post("http://localhost:3001/push")
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Push failed {:?}", resp));
    }

    // for part in parts {
    //     form = form.part(generate_id(), part);
    // }

    //let client = reqwest::Client::builder(

    Ok(())
}
