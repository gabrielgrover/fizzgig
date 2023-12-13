use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::HeaderValue, request::Parts, StatusCode},
};

pub struct BearerTokenExtractor(HeaderValue);

const AUTHORIZATION: &str = "Authorization";

impl BearerTokenExtractor {
    pub fn to_str(&self) -> Result<&str, axum::http::header::ToStrError> {
        self.0.to_str()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for BearerTokenExtractor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(header_val) = parts.headers.get(AUTHORIZATION) {
            let bearer_token_str = header_val
                .to_str()
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid bearer token"))?;

            let is_bearer = bearer_token_str.starts_with("Bearer ");

            if !is_bearer {
                return Err((StatusCode::BAD_REQUEST, "Invalid bearer token"));
            }

            let token_str = &bearer_token_str[7..];
            let token_header_val = HeaderValue::from_str(token_str)
                .map_err(|_e| (StatusCode::BAD_REQUEST, "Invalid bearer token"))?;

            Ok(BearerTokenExtractor(token_header_val))
        } else {
            Err((StatusCode::BAD_REQUEST, "Authorization header is missing"))
        }
    }
}
