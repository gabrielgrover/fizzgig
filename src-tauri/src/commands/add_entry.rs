use super::SavedPassword;
use local_ledger::{LocalLedger, LocalLedgerError};

#[tauri::command]
pub fn add_entry(
    collection_name: String,
    master_pass: String,
    entry_name: String,
    val: String,
) -> Result<(), LocalLedgerError> {
    let mut ledger = LocalLedger::<SavedPassword>::new(&collection_name, master_pass)?;

    let _ = ledger.create(
        SavedPassword {
            pw: val,
            name: entry_name.clone(),
        },
        &entry_name,
    )?;

    Ok(())
}
