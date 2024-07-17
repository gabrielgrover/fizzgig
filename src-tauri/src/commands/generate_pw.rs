use passwords::PasswordGenerator;

#[tauri::command]
pub async fn generate_pw<'a>() -> Result<String, String> {
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

    Ok(pw)
}
