use passwords::PasswordGenerator;

use crate::llw_handler::LocalLedgerWorkerHandler;

#[tauri::command]
pub async fn regen_pw<'a>(
    entry_name: String,
    state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<(), String> {
    let pg = PasswordGenerator {
        length: 10,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: false,
    };

    let pw = pg.generate_one().map_err(|err| err.to_string())?;

    state
        .update_entry(&entry_name, &pw)
        .await
        .map_err(|err| err.to_string())
}
