mod lib;

use axum::{Router, response::{Html, Json}, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::lib::token::{AuthError, Token, generate_token_set};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let app = Router::new().route("/", get(handler)).route("/authorize", post(authorize)).route("/protected", get(protected));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
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

    let token = Token {
        sub: "b@b.com".to_owned(),
    };
    let (access_token, refresh_token) = generate_token_set(&token)?;

    Ok(Json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

async fn protected(token: Token) -> Result<String, AuthError> {
    Ok(format!("Welcome to the protected area :)\nYour data: \n{token}"))
} 


