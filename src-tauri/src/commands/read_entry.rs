use local_ledger::{LocalLedger, LocalLedgerError};

use super::SavedPassword;

#[tauri::command]
pub fn read_entry(
    collection_name: String,
    master_pass: String,
    entry_name: String,
) -> Result<String, LocalLedgerError> {
    let mut ledger = LocalLedger::<SavedPassword>::new(&collection_name, master_pass)?;

    let saved_password = ledger.read_by_entry_name(&entry_name)?;

    Ok(saved_password.pw.clone())
}
