use std::{fmt::Display, sync::LazyLock};

use axum::{Json, RequestPartsExt, extract::FromRequestParts, http::StatusCode, response::IntoResponse};
use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;


static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = std::env::var("ACCESS_SECRET").expect("ACCESS_SECRET must be set");
    let secret = secret.as_bytes();
    Keys {
        access_encoding: EncodingKey::from_secret(secret),
        access_decoding: DecodingKey::from_secret(secret),
        refresh_encoding: EncodingKey::from_secret(secret),
        refresh_decoding: DecodingKey::from_secret(secret),
    }
});

struct Keys {
    access_encoding: EncodingKey,
    access_decoding: DecodingKey,
    refresh_encoding: EncodingKey,
    refresh_decoding: DecodingKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub sub: String
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}", self.sub)
    }
}

impl<S> FromRequestParts<S> for Token where S: Send + Sync {
    type Rejection = AuthError;

    async fn from_request_parts(
            parts: &mut axum::http::request::Parts,
            _state: &S,
        ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts.extract::<TypedHeader<Authorization<Bearer>>>().await.map_err(|_| AuthError::InvalidToken)?;
        let token_data = decode::<Token>(bearer.token(), &KEYS.access_decoding, &Validation::default()).map_err(|_| AuthError::InvalidToken)?;
        
        Ok(token_data.claims)
    }
}

pub fn generate_token_set(token: &Token) -> Result<(String, String), AuthError> {
    let access_token = encode(&Header::default(), token, &KEYS.access_encoding).map_err(|_| AuthError::TokenCreation)?;
    let refresh_token = encode(&Header::default(), &RefreshClaims { sub: token.sub.clone()}, &KEYS.refresh_encoding).map_err(|_| AuthError::TokenCreation)?;
    
    Ok((access_token, refresh_token))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshClaims {
    sub: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessClaims {
    sub: String
}


#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Toekn creation error"),
            AuthError::WrongCredentials => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token")
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}