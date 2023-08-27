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

    let mut pw_hashed = false;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().map_or(
            Err((
                StatusCode::BAD_REQUEST,
                "Form fields must have a name".to_string(),
            )),
            |i| Ok(i.to_string()),
        )?;

        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        let d = if name == "pw" {
            pw_hashed = true;
            pwhash::bcrypt::hash(data)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .into()
        } else {
            data
        };

        sync_job.insert(name, d);
    }

    if !pw_hashed {
        return Err((
            StatusCode::BAD_REQUEST,
            "Expected form field `pw`".to_string(),
        ));
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
    pub pw: String,
}

async fn pull(State(state): State<AppState>, params: Query<PullReqParams>) -> impl IntoResponse {
    let pin = &params.pin;
    let pass = &params.pw;
    let mut jobs = state.sync_jobs.lock().await;
    let boundary = format!("\r\n---{}\r\n", generate_id());
    let sync_job = jobs.remove(pin).map_or(
        Err((StatusCode::NOT_FOUND, format!("No data found for {}", pin))),
        Ok,
    )?;

    let hashed_pw = sync_job
        .get("pw")
        .map_or(
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password not found for job".to_string(),
            )),
            Ok,
        )
        .and_then(|b| {
            std::str::from_utf8(b).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        })?;

    let correct_pw = pwhash::bcrypt::verify(pass, hashed_pw);

    if !correct_pw {
        jobs.insert(pin.to_string(), sync_job);

        return Err((StatusCode::BAD_REQUEST, "invalid pw".to_string()));
    }

    let mut parts: Vec<Result<Bytes, Infallible>> = vec![];

    for (k, bytes) in sync_job.into_iter() {
        if k != "pw" {
            parts.push(Ok(bytes));
            parts.push(Ok(boundary.clone().into()))
        }
    }

    let s = stream::iter(parts);
    let sb = StreamBody::new(s);

    Ok((StatusCode::OK, sb))
}

#[derive(Debug, Clone)]
struct AppState {
    pub sync_jobs: Arc<Mutex<SyncJobs>>,
}

type SyncJobs = HashMap<String, HashMap<String, Bytes>>;
