use crate::LandStriderConfig;
use reqwest::multipart::Form;
use std::error::Error;

const END_POINT: &str = "/push";

#[derive(Debug, thiserror::Error)]
enum PushErr {
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("Server responded with a failure: {0}")]
    Server(String),
    #[error("Failed to parse push response: {0}")]
    Parse(String),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PushResponse {
    success: bool,
    pin: String,
}

pub async fn push(
    land_strider_config: &LandStriderConfig,
    f: Form,
) -> Result<PushResponse, Box<dyn Error>> {
    let base_url = land_strider_config.get_base_url();
    let url = format!("{}{}", base_url, END_POINT);

    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .multipart(f)
        .send()
        .await
        .map_err(|e| PushErr::Network(e.to_string()))?;

    if !resp.status().is_success() {
        let server_msg = resp.text().await?;
        let err = Box::new(PushErr::Server(server_msg));

        return Err(err);
    }

    let data = resp.text().await?;
    let push_resp: PushResponse =
        serde_json::from_str(&data).map_err(|e| PushErr::Parse(e.to_string()))?;

    Ok(push_resp)
}
