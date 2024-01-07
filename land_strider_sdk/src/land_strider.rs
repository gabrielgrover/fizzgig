use crate::{pull_stream::PullStream, LandStriderConfig};
use reqwest::multipart::Form;
use std::error::Error;

pub struct LandStrider {
    config: LandStriderConfig,
}

impl LandStrider {
    pub fn new(config: LandStriderConfig) -> Self {
        Self { config }
    }

    pub async fn push(&self, f: Form) -> Result<crate::push::PushResponse, Box<dyn Error>> {
        crate::push::push(&self.config, f).await
    }

    pub async fn pull_s(&self, pin: &str, pw: &str) -> Result<PullStream, Box<dyn Error>> {
        crate::pull_s::pull_s(&self.config, pin, pw).await
    }

    pub async fn push_s<R: std::io::Read + Send + Sync + 'static>(
        &self,
        r: R,
        pw: String,
    ) -> Result<crate::push::PushResponse, Box<dyn Error>> {
        crate::push::push_s(&self.config, r, pw).await
    }
}
