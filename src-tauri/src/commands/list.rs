use local_ledger::{LocalLedger, LocalLedgerError};

use super::SavedPassword;

#[tauri::command]
pub fn list(collection_name: String, master_pass: String) -> Result<Vec<String>, LocalLedgerError> {
    let local_ledger = LocalLedger::<SavedPassword>::new(&collection_name, master_pass)?;

    let pw_entry_list = local_ledger.list_entry_labels()?;

    Ok(pw_entry_list
        .into_iter()
        .map(|label| label.to_owned())
        .collect::<Vec<String>>())
}
