use axum_server::app::create_app;


#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").expect("PORT must be set");
    
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
    
    println!("listening on {}", listener.local_addr().unwrap());
    
    axum::serve(listener, create_app().await).await.unwrap();
}