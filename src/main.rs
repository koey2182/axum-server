use axum::{Router, response::{Html, Json}, routing::{get, post}};
use axum_server::token::{AccessClaims, AccessInfo, AuthError, RefreshClaims, RefreshInfo, generate_access_token, generate_refresh_token};
use serde::Deserialize;
use serde_json::{Value, json};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let app = Router::new()
        .route("/", get(handler))
        .route("/authorize", post(authorize))
        .route("/protected", get(protected))
        .route("/refresh", post(refresh));

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

    let access_claims = AccessClaims::new("foo".to_owned());
    let access_token = generate_access_token(access_claims)?;
    let refresh_claims = RefreshClaims::new("foo".to_owned());
    let refresh_token = generate_refresh_token(refresh_claims)?;

    Ok(Json(json!({"access_token": access_token, "refresh_token": refresh_token})))
}

async fn protected(access_info: AccessInfo) -> Result<String, AuthError> {
    Ok(format!("Welcome to the protected area :)\nYour data: {}\n", access_info.user_id))
} 

async fn refresh(refresh_info: RefreshInfo) -> Result<String, AuthError> {
    let claims = AccessClaims::new(refresh_info.user_id);
    generate_access_token(claims)
}

