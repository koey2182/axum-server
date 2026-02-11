use axum::Router;
use sqlx::PgPool;

use crate::{api, state::AppState};

pub async fn create_app() -> Router {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set")).await.expect("postgres is not ready");
    let state = AppState { pool };

    Router::new()
        .nest("/api", api::route())
        .with_state(state)
}
