use crate::LandStriderConfig;
use futures::StreamExt;
use reqwest::multipart::Form;
use std::error::Error;

const PUSH_END_POINT: &str = "/push";
const PUSH_S_END_POINT: &str = "/push_s";
const RESERVE_PIN_END_POINT: &str = "/reserve_pin";

#[derive(Debug, thiserror::Error)]
pub enum PushErr {
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

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ReservePinResponse {
    success: bool,
    token: String,
    pin: String,
}

pub async fn push(
    land_strider_config: &LandStriderConfig,
    f: Form,
) -> Result<PushResponse, Box<dyn Error>> {
    let base_url = land_strider_config.get_base_url();
    let url = format!("{}{}", base_url, PUSH_END_POINT);
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

const READER_RETRY_MAX: usize = 3;

pub async fn push_s<R: std::io::Read + Sync + Send + 'static>(
    land_strider_config: &LandStriderConfig,
    mut r: R,
    pw: String,
) -> Result<PushResponse, Box<dyn Error>> {
    let base_url = land_strider_config.get_base_url();
    let push_url = format!("{}{}", base_url, PUSH_S_END_POINT);
    let reserve_url = format!("{}{}", base_url, RESERVE_PIN_END_POINT);
    let client = reqwest::Client::new();
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tracing::info!("Reserving pin...");

    let reserve_resp = client
        .post(&reserve_url)
        .json(&serde_json::json!({"pw": pw}))
        .send()
        .await
        .map_err(|e| PushErr::Server(e.to_string()))?;

    let data = reserve_resp.text().await?;
    let reserve_pin_resp: ReservePinResponse =
        serde_json::from_str(&data).map_err(|e| PushErr::Parse(e.to_string()))?;

    tokio::task::spawn_blocking(move || {
        let mut buf = [0; 1024];
        let mut retry: usize = 0;

        tracing::info!("Starting to read push data");

        loop {
            match r.read(&mut buf) {
                Ok(count) => {
                    if count == 0 && retry >= READER_RETRY_MAX {
                        tracing::info!("Read finished");
                        break;
                    }

                    if count == 0 && retry < READER_RETRY_MAX {
                        retry += 1;
                        continue;
                    }

                    retry = 0;

                    if tx.blocking_send(buf[..count].to_vec()).is_err() {
                        tracing::error!("Failed to send read data to channel");
                        break;
                    }
                }

                Err(e) => {
                    tracing::error!("Read failed: {}", e);

                    break;
                }
            }
        }
    });

    let s = tokio_stream::wrappers::ReceiverStream::new(rx).map(|r| Ok::<_, reqwest::Error>(r));
    let resp = client
        .post(&push_url)
        .header(
            "Authorization",
            format!("Bearer {}", reserve_pin_resp.token),
        )
        .header("Content-type", "application/octet-stream")
        .body(reqwest::Body::wrap_stream(s))
        .send()
        .await
        .map_err(|e| PushErr::Server(e.to_string()))?;

    if !resp.status().is_success() {
        let server_msg = resp.text().await?;
        let err = Box::new(PushErr::Server(server_msg));

        return Err(err);
    }

    let _ = resp.text().await?;
    let push_resp = PushResponse {
        success: true,
        pin: reserve_pin_resp.pin,
    };

    Ok(push_resp)
}
