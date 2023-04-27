use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn export_ledger<'a>(
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), String> {
    let source_dir = state
        .get_ledger_dir()
        .await
        .map_err(|err| err.to_string())?;

    let file_name = "ledger.zip";

    if let Some(mut desktop_path) = dirs::desktop_dir() {
        desktop_path.push(file_name);
        let output_file = desktop_path.to_str().unwrap();

        zip_directory(&source_dir, output_file).map_err(|err| err.to_string())
    } else {
        Err("Failed to export ledger".to_string())
    }
}

fn zip_directory(
    source_path: &PathBuf,
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    //let source_path = Path::new(source_dir);
    let output = File::create(output_file)?;
    let mut zip = ZipWriter::new(output);

    for entry in WalkDir::new(source_path) {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(source_path)?;
        let mut clean_path = PathBuf::new();

        for component in relative_path.components() {
            clean_path.push(component);
        }

        if entry.file_type().is_file() {
            let mut file = File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            let options = if cfg!(unix) {
                let permissions = entry.metadata()?.mode();
                FileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(permissions)
            } else {
                // On Windows, we don't set custom permissions.
                FileOptions::default().compression_method(CompressionMethod::Deflated)
            };

            zip.start_file(clean_path.to_str().unwrap(), options)?;
            zip.write_all(&*buffer)?;
        }
    }

    zip.finish()?;
    Ok(())
}
