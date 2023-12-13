use secrecy::Secret;

#[derive(Debug, Clone)]
pub struct ApplicationSettings {
    pin_secret: String,
}

impl ApplicationSettings {
    pub fn jwt_pin_secret(&self) -> Secret<String> {
        Secret::new(self.pin_secret.clone())
    }
}

pub fn get_app_config() -> ApplicationSettings {
    match std::env::var("PIN_SECRET") {
        Ok(pin_secret) => ApplicationSettings { pin_secret },
        Err(e) => {
            tracing::warn!("PIN_SECRET not found: {:?}", e);

            ApplicationSettings {
                pin_secret: "LAND_STRIDER_PIN_SECRET".to_string(),
            }
        }
    }
}
