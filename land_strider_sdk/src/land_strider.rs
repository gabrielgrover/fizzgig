use std::error::Error;

use reqwest::multipart::Form;

use crate::{push::*, LandStriderConfig};

pub struct LandStrider {
    config: LandStriderConfig,
}

impl LandStrider {
    pub fn new(config: LandStriderConfig) -> Self {
        Self { config }
    }

    pub async fn push(&self, f: Form) -> Result<PushResponse, Box<dyn Error>> {
        push(&self.config, f).await
    }
}
