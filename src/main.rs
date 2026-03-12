use axum_server::{app::create_app, mqtt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").expect("PORT must be set");
    let mqtt_port: u16 = std::env::var("MQTT_PORT")
        .unwrap_or_else(|_| "1883".to_owned())
        .parse()
        .expect("MQTT_PORT must be a valid port number");

    let router = mqtt::Router::new();
    tokio::spawn(mqtt::serve(mqtt_port, router));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    println!("[http] 서버 시작: {}", listener.local_addr().unwrap());

    axum::serve(listener, create_app().await).await.unwrap();
}