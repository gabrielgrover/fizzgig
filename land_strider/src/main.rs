use axum::{
    body::{Bytes, StreamBody},
    extract::{Multipart, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use futures_util::stream;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use utility::{generate_id, generate_pin};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        sync_jobs: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/push", post(push))
        .route("/pull", get(pull))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn push(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let pin = generate_pin();
    let mut sync_job = HashMap::<String, Bytes>::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        sync_job.insert(name, data);
    }

    state.sync_jobs.lock().await.insert(pin.clone(), sync_job);

    tracing::info!("CREATED: {}", pin);

    Ok((
        StatusCode::CREATED,
        serde_json::json!({"success": true, "pin": pin}).to_string(),
    ))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PullReqParams {
    pub pin: String,
}

async fn pull(State(state): State<AppState>, params: Query<PullReqParams>) -> impl IntoResponse {
    let pin = &params.pin;
    let maybe_data = state.sync_jobs.lock().await.remove(pin);
    let boundary = format!("\r\n---{}\r\n", generate_id());

    match maybe_data {
        Some(sync_job) => {
            let parts = sync_job.into_values().fold(
                vec![],
                |mut a: Vec<Result<Bytes, Infallible>>, bytes| {
                    a.push(Ok(bytes));
                    a.push(Ok(boundary.clone().into()));

                    a
                },
            );

            let s = stream::iter(parts);
            let sb = StreamBody::new(s);

            Ok((StatusCode::OK, sb))
        }

        None => Err((StatusCode::NOT_FOUND, format!("No data found for {}", pin))),
    }
}

#[derive(Debug, Clone)]
struct AppState {
    pub sync_jobs: Arc<Mutex<SyncJobs>>,
}

type SyncJobs = HashMap<String, HashMap<String, Bytes>>;
