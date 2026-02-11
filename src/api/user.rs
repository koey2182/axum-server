use axum::{Router, extract::State, routing::get};

use crate::{state::AppState, token::AuthError};

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/users", get(users))
}

async fn users(State(AppState {pool}): State<AppState>) -> Result<String, AuthError> {
    let row: (String,) = sqlx::query_as("SELECT '홍길동'").fetch_one(&pool).await.map_err(|_| AuthError::WrongCredentials)?;
    Ok(row.0)
}