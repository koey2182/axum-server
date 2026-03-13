use axum::Router;

use crate::{api, state::AppState};

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .nest("/api", api::route())
        .with_state(state)
}
