use axum::{Json, Router, body::Body, extract::State, http::{Response, StatusCode}, response::IntoResponse, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{state::AppState, token::{AccessClaims, AuthError, RefreshClaims, generate_access_token, generate_refresh_token}};

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
    let refresh_claims = RefreshClaims::new("foo".to_owned()).await;
    let refresh_token = generate_refresh_token(refresh_claims)?;

    Ok(Json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

async fn protected(claims: AccessClaims) -> Result<String, AuthError> {
    Ok(format!("Welcome to the protected area :)\nYour data: {}\n", claims.user_id))
} 

pub async fn refresh(refresh_claims: RefreshClaims, State(AppState{pool}): State<AppState>) -> Response<Body> {
    let db_refresh_token = sqlx::query!(r#"SELECT * FROM refresh_tokens rt WHERE rt.jti = $1;"#, refresh_claims.jti)
        .fetch_optional(&pool).await;

    if let Err(e) = db_refresh_token {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "INTERNAL_SERVER_ERROR", "message": e.to_string()}))).into_response();
    }

    match db_refresh_token.unwrap() {
        Some(token) => {
            let access_token =  generate_access_token(AccessClaims::new(refresh_claims.user_id));
            let refresh_token = generate_refresh_token(RefreshClaims::new(token.owner_id).await);
            (StatusCode::OK, Json(json!({"access_token": access_token, "refresh_token": refresh_token}))).into_response()
        },
        None => (StatusCode::UNAUTHORIZED, Json(json!({"error": "NOT_MANAGED_REFRESH_TOKEN", "message": "관리되지 않는 리프레쉬토큰"}))).into_response()
    }
}