use crate::{pull_stream::*, LandStriderConfig};
use std::error::Error;

#[derive(Debug, thiserror::Error)]
enum PullErr {
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("Server responded with a failure: {0}")]
    Server(String),
}

const PULL_END_POINT: &str = "/pull";

pub async fn pull_s(
    land_strider_config: &LandStriderConfig,
    pin: &str,
    pw: &str,
) -> Result<PullStream, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let base_url = land_strider_config.get_base_url();
    let url = format!("{}{}?pin={}&pw={}", base_url, PULL_END_POINT, pin, pw);
    tracing::info!("Sending pull request");
    let response: reqwest::Response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| PullErr::Network(e.to_string()))?;

    if !response.status().is_success() {
        let server_msg = response.text().await?;
        let err = Box::new(PullErr::Server(server_msg));

        return Err(err);
    }

    tracing::info!("Start response stream parse");

    let pull_stream: PullStream = response.into();

    Ok(pull_stream)
}
