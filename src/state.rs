use std::sync::Arc;

use sqlx::PgPool;

use crate::mqtt::BrokerState;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub mqtt: Arc<BrokerState>,
}