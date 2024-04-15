use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use futures_util::stream;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use utility::generate_pin;

use crate::*;

const PUSH_READY_STATUS: &str = "PUSH_READY";
const PULL_READY_STATUS: &str = "PULL_READY";

pub async fn run(host: &str, port: &str) {
    let app = land_strider_app();
    let url = format!("{}:{}", host, port);

    tracing::info!("listening on {}", url);
    let listener = tokio::net::TcpListener::bind(url).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub fn land_strider_app() -> Router {
    let state = AppState {
        sync_jobs: Arc::new(Mutex::new(HashMap::new())),
        app_settings: get_app_config(),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/pull", get(pull))
        .route("/push_s", post(push_s))
        .route("/reserve_pin", post(reserve_pin))
        .with_state(state);

    app
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn push_s(
    State(state): State<AppState>,
    token: BearerTokenExtractor,
    b: axum::body::Body, //mut s: BodyStream,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let token_str = token.to_str().map_err(|e| {
        tracing::error!("Failed to convert token to &str: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid token".to_string(),
        )
    })?;
    let claims = validate_jwt(&state.app_settings, token_str).map_err(|e| {
        tracing::error!("Failed to validate jwt: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Token validation failed".to_string(),
        )
    })?;

    let mut jobs = state.sync_jobs.lock().await;
    let sync_job = jobs
        .get_mut(&claims.pin)
        .ok_or((StatusCode::BAD_REQUEST, "Token no longer valid".to_string()))?;

    let is_push_ready = sync_job
        .get("status")
        .ok_or((StatusCode::BAD_REQUEST, "Token no longer valid".to_string()))
        .and_then(|b| {
            std::str::from_utf8(&b)
                .map(|status| status == PUSH_READY_STATUS)
                .map_err(|_e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Upload failed".to_string(),
                    )
                })
        })?;

    if !is_push_ready {
        return Err((
            StatusCode::BAD_REQUEST,
            "Push token already used.  Please reserve a new one".to_string(),
        ));
    }

    sync_job.insert("status".to_string(), PULL_READY_STATUS.into());

    let mut buf = Vec::new();

    tracing::info!("Start stream processing");

    let mut s = b.into_data_stream();

    while let Some(chunk) = s.next().await {
        let chunk = chunk.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to process push stream: {:?}", e),
            )
        })?;

        buf.extend_from_slice(&chunk);
    }

    sync_job.insert("data".into(), buf.into());

    tracing::info!("Job {} is pull ready", &claims.pin);

    Ok((
        StatusCode::CREATED,
        serde_json::json!({"success": true}).to_string(),
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
    let sync_job = jobs
        .remove(pin)
        .ok_or((StatusCode::BAD_REQUEST, "invalid credentials".to_string()))?;
    let hashed_pw = sync_job
        .get("pw")
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password not found for job".to_string(),
        ))
        .and_then(|b| {
            std::str::from_utf8(b).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        })?;
    let correct_pw = pwhash::bcrypt::verify(pass, hashed_pw);

    if !correct_pw {
        jobs.insert(pin.to_string(), sync_job);

        return Err((StatusCode::BAD_REQUEST, "invalid credentials".to_string()));
    }

    let mut parts: Vec<Result<Bytes, Infallible>> = vec![];

    for (k, bytes) in sync_job.into_iter() {
        if k == "pw" {
            continue;
        }

        if k == "status" {
            let status = std::str::from_utf8(&bytes).map_err(|err| {
                tracing::error!("Failed to convert status to utf8: {}", err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "There was an error processing your request.  Please try again later."
                        .to_string(),
                )
            })?;

            if status != PULL_READY_STATUS {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Data is not ready.  It is possible it has already been pulled.".to_string(),
                ));
            }

            continue;
        }

        parts.push(Ok(bytes));
        parts.push(Ok("\n\n".into()));
    }

    let s = stream::iter(parts);
    let sb = axum::body::Body::from_stream(s);

    Ok((StatusCode::OK, sb))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReservePinReqPayload {
    pw: String,
}

async fn reserve_pin(
    State(state): State<AppState>,
    Json(payload): Json<ReservePinReqPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    tracing::info!("Reserving pin");
    let pin = generate_pin();
    let mut sync_job = HashMap::<String, Bytes>::new();
    let pw_hash = pwhash::bcrypt::hash(payload.pw)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .into();

    sync_job.insert("pw".to_string(), pw_hash);
    sync_job.insert("status".to_string(), PUSH_READY_STATUS.into());

    let jwt = gen_jwt(&state.app_settings, pin.clone()).map_err(|e| {
        tracing::error!("Failed to to generate jwt: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to reserve pin".to_string(),
        )
    })?;

    state.sync_jobs.lock().await.insert(pin.clone(), sync_job);

    tracing::info!("Pin reserverd: {}", pin);

    Ok((
        StatusCode::CREATED,
        serde_json::json!({"success": true, "token": jwt, "pin": pin}).to_string(),
    ))
}

// async fn push(
//     State(state): State<AppState>,
//     mut multipart: Multipart,
// ) -> Result<(StatusCode, String), (StatusCode, String)> {
//     let pin = generate_pin();
//     let mut sync_job = HashMap::<String, Bytes>::new();
//
//     let mut pw_hashed = false;
//
//     while let Some(field) = multipart
//         .next_field()
//         .await
//         .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
//     {
//         let name = field
//             .name()
//             .ok_or((
//                 StatusCode::BAD_REQUEST,
//                 "Form fields must have a name".to_string(),
//             ))?
//             .to_string();
//
//         let data = field
//             .bytes()
//             .await
//             .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
//
//         let d = if name == "pw" {
//             pw_hashed = true;
//             pwhash::bcrypt::hash(data)
//                 .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
//                 .into()
//         } else {
//             data
//         };
//
//         sync_job.insert(name, d);
//     }
//
//     if !pw_hashed {
//         return Err((
//             StatusCode::BAD_REQUEST,
//             "Expected form field `pw`".to_string(),
//         ));
//     }
//
//     state.sync_jobs.lock().await.insert(pin.clone(), sync_job);
//
//     Ok((
//         StatusCode::CREATED,
//         serde_json::json!({"success": true, "pin": pin}).to_string(),
//     ))
// }

#[derive(Debug, Serialize, Deserialize)]
struct JWTClaims {
    pin: String,
    exp: usize,
}

fn gen_jwt(settings: &ApplicationSettings, pin: String) -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_e| {
            tracing::error!("Failed to get system time for jwt expiry");
            "Failed to reserve pin".to_string()
        })?
        .as_secs();
    let time_stamp = now + 86_400;
    let jwt_secret = settings.jwt_pin_secret();
    let claims = JWTClaims {
        pin,
        exp: time_stamp as usize,
    };
    let key = EncodingKey::from_secret(jwt_secret.expose_secret().as_ref());
    let token = encode(&Header::default(), &claims, &key);

    token.map_err(|e| e.to_string())
}

fn validate_jwt(settings: &ApplicationSettings, token: &str) -> Result<JWTClaims, String> {
    let jwt_secret = settings.jwt_pin_secret();
    let decode_key = DecodingKey::from_secret(jwt_secret.expose_secret().as_ref());
    let decoded_token = decode::<JWTClaims>(token, &decode_key, &Validation::default())
        .map_err(|e| e.to_string())?;

    Ok(decoded_token.claims)
}

#[derive(Debug, Clone)]
struct AppState {
    pub sync_jobs: Arc<Mutex<SyncJobs>>,
    pub app_settings: ApplicationSettings,
}

type SyncJobs = HashMap<String, HashMap<String, Bytes>>;
