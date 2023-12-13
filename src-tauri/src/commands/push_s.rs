use crate::llw_handler::LocalLedgerWorkerHandler;
use land_strider_sdk::{LandStrider, PushResponse};

#[tauri::command]
pub async fn push_s<'a>(
    temp_pw: String,
    llw: tauri::State<'a, LocalLedgerWorkerHandler>,
    land_strider: tauri::State<'a, LandStrider>,
) -> Result<PushResponse, String> {
    let ledger_dump = llw.get_doc_dump().await.map_err(|e| {
        tracing::error!("Failed to get doc dump {:?}", e);
        e.to_string()
    })?;

    land_strider
        .push_s(ledger_dump, temp_pw)
        .await
        .map_err(|e| {
            tracing::error!("land_strider push failed: {:?}", e);
            e.to_string()
        })
}
