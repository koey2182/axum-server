use std::sync::LazyLock;

use axum::{Json, RequestPartsExt, extract::{FromRequestParts}, http::StatusCode, response::IntoResponse};
use axum_extra::{TypedHeader, headers::{Authorization, authorization::Bearer}};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::id::ulid;


static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let access_secret = std::env::var("ACCESS_SECRET").expect("ACCESS_SECRET must be set");
    let access_secret = access_secret.as_bytes();
    let refresh_secret = std::env::var("REFRESH_SECRET").expect("REFRESH_SECRET must be set");
    let refresh_secret = refresh_secret.as_bytes();
    Keys {
        access_encoding: EncodingKey::from_secret(access_secret),
        access_decoding: DecodingKey::from_secret(access_secret),
        refresh_encoding: EncodingKey::from_secret(refresh_secret),
        refresh_decoding: DecodingKey::from_secret(refresh_secret),
    }
});

static TOKEN_LIFE: LazyLock<TokenLife> = LazyLock::new(|| {
    let access_minutes = std::env::var("ACCESS_MINUTES").expect("ACCESS_MINUTES must be set").parse::<i64>().expect("ACCESS_MINUTES is not integer");
    let refresh_days = std::env::var("REFRESH_DAYS").expect("REFRESH_DAYS must be set").parse::<i64>().expect("REFRESH_DAYS is not integer");
    TokenLife { access_minutes, refresh_days }
});

struct TokenLife {
    access_minutes: i64,
    refresh_days: i64
}

struct Keys {
    access_encoding: EncodingKey,
    access_decoding: DecodingKey,
    refresh_encoding: EncodingKey,
    refresh_decoding: DecodingKey,
}

pub fn generate_access_token(claims: AccessClaims) -> Result<String, AuthError> {
    let access_token = encode(&Header::default(), &claims, &KEYS.access_encoding).map_err(|_| AuthError::TokenCreation)?;
    Ok(access_token)
}

pub fn generate_refresh_token(claims: RefreshClaims)-> Result<String, AuthError> {
    let refresh_token = encode(&Header::default(), &claims, &KEYS.refresh_encoding).map_err(|_| AuthError::TokenCreation)?;
    Ok(refresh_token)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub user_id: String,
    exp: usize,
    iat: usize
}
impl AccessClaims {
    pub fn new(user_id: String) -> Self {
        let iat = Utc::now();
        let exp = iat + Duration::minutes(TOKEN_LIFE.access_minutes);
        Self {
            user_id,
            exp: usize::try_from(exp.timestamp()).expect("exp is not usize"),
            iat: usize::try_from(iat.timestamp()).expect("iat is not usize")
        }
    }
}
impl<S> FromRequestParts<S> for AccessClaims where S: Send + Sync {
    type Rejection = AuthError;

    async fn from_request_parts(
            parts: &mut axum::http::request::Parts,
            _state: &S,
        ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts.extract::<TypedHeader<Authorization<Bearer>>>().await.map_err(|_| AuthError::InvalidToken)?;
        let token_data = decode::<AccessClaims>(bearer.token(), &KEYS.access_decoding, &Validation::default()).map_err(|err| {println!("{err}"); AuthError::InvalidToken})?;
        Ok(token_data.claims)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub jti: String,
    pub user_id: String,
    exp: usize,
    iat: usize
}
impl RefreshClaims {
    pub async fn new(user_id: String) -> Self {
        let jti= ulid().await;
        let iat = Utc::now();
        let exp = iat + Duration::days(TOKEN_LIFE.refresh_days);
        Self {
            jti,
            user_id,
            exp: usize::try_from(exp.timestamp()).expect("exp is not usize"),
            iat: usize::try_from(iat.timestamp()).expect("iat is not usize")
        }
    }
}
impl <S> FromRequestParts<S> for RefreshClaims where  S: Send + Sync {
    type Rejection = AuthError;

    async fn from_request_parts(
            parts: &mut axum::http::request::Parts,
            _state: &S,
        ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts.extract::<TypedHeader<Authorization<Bearer>>>().await.map_err(|_| AuthError::InvalidToken)?;
        let token_data = decode::<RefreshClaims>(bearer.token(), &KEYS.refresh_decoding, &Validation::default()).map_err(|_| AuthError::InvalidToken)?;
        Ok(token_data.claims)
    }
}

#[derive(Debug, Serialize)]
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