use crate::LandStriderConfig;
use serde_json::Value;
use std::error::Error;
use tokio_stream::StreamExt;

#[derive(Debug, thiserror::Error)]
enum PullErr {
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("Server responded with a failure: {0}")]
    Server(String),
    #[error("Failed to process stream item: {0}")]
    StreamItemProcess(String),
    #[error("Failed to parse stream data: {0}")]
    StreamItemParse(String),
}

const PULL_END_POINT: &str = "/pull";

pub async fn pull(
    land_strider_config: &LandStriderConfig,
    pin: &str,
    pw: &str,
) -> Result<Vec<Value>, Box<dyn Error>> {
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

    let mut s = response.bytes_stream();
    let mut buf = vec![];
    let mut values = vec![];

    while let Some(item) = s.next().await {
        let bytes = item.map_err(|e| PullErr::StreamItemProcess(e.to_string()))?;

        for byte in bytes {
            //tracing::info!("buffer: {}", std::str::from_utf8(buf.as_ref()).unwrap());
            let received_new_line = byte == b'\n';

            if received_new_line && !buf.is_empty() {
                let slice = buf.as_slice();
                let v: Value = serde_json::from_slice(slice)
                    .map_err(|e| PullErr::StreamItemParse(e.to_string()))?;

                values.push(v);
                buf.clear();
            } else {
                buf.push(byte);
            }
        }
    }

    Ok(values)
}
