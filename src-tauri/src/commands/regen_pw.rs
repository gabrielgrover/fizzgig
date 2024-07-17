use crate::app_state::AppState;
use passwords::PasswordGenerator;

#[tauri::command]
pub async fn regen_pw<'a>(
    entry_name: String,
    app_state: tauri::State<'a, AppState>,
) -> Result<(), String> {
    let pg = PasswordGenerator {
        length: 20,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: false,
    };

    let pw = pg.generate_one().map_err(|err| err.to_string())?;

    app_state
        .pw_ledger
        .lock()
        .await
        .update_entry(&entry_name, &pw)
}
