use crate::llw_handler::LocalLedgerWorkerHandler;
use passwords::PasswordGenerator;

#[tauri::command]
pub async fn generate_pw<'a>(
    _state: tauri::State<'a, LocalLedgerWorkerHandler>,
) -> Result<String, String> {
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

    Ok(pw)
}
