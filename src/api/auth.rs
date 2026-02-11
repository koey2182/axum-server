use axum::{Json, Router, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{state::AppState, token::{AccessClaims, AccessInfo, AuthError, RefreshClaims, RefreshInfo, generate_access_token, generate_refresh_token}};

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/auth/refresh", post(refresh))
        .route("/auth/authorize", post(authorize))
        .route("/auth/protected", get(protected))
}

#[derive(Debug, Deserialize)]
struct AuthPayload {
    client_id: String,
    client_secret: String,
}
async fn authorize(Json(payload): Json<AuthPayload>) -> Result<Json<Value>, AuthError> {
    if payload.client_id.is_empty() || payload.client_secret.is_empty() {
        return Err(AuthError::MissingCredentials);
    }

    if payload.client_id != "foo" || payload.client_secret != "bar" {
        return Err(AuthError::WrongCredentials);
    }

    let access_claims = AccessClaims::new("foo".to_owned());
    let access_token = generate_access_token(access_claims)?;
    let refresh_claims = RefreshClaims::new("foo".to_owned());
    let refresh_token = generate_refresh_token(refresh_claims)?;

    Ok(Json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

async fn protected(access_info: AccessInfo) -> Result<String, AuthError> {
    Ok(format!("Welcome to the protected area :)\nYour data: {}\n", access_info.user_id))
} 

pub async fn refresh(refresh_info: RefreshInfo) -> Result<String, AuthError> {
    let claims = AccessClaims::new(refresh_info.user_id);
    generate_access_token(claims)
}