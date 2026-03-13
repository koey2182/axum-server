use std::sync::Arc;

use axum_server::{app::create_app, handlers, mqtt, state::AppState};
use sqlx::PgPool;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").expect("PORT must be set");
    let mqtt_port: u16 = std::env::var("MQTT_PORT")
        .unwrap_or_else(|_| "1883".to_owned())
        .parse()
        .expect("MQTT_PORT must be a valid port number");

    let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("postgres is not ready");

    let mqtt = mqtt::new_state();
    tokio::spawn(mqtt::serve(Arc::clone(&mqtt), mqtt_port, handlers! {}));

    let state = AppState { pool, mqtt };

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    println!("[http] 서버 시작: {}", listener.local_addr().unwrap());

    axum::serve(listener, create_app(state)).await.unwrap();
}