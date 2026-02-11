use axum::Router;

use crate::state::AppState;

mod user;
mod auth;

pub fn route() -> Router<AppState> {
    Router::new()
        .merge(user::route())
        .merge(auth::route())
}