use axum::{Router, extract::State, response::{Html, Json}, routing::{get, post}};
use axum_server::token::{AccessClaims, AccessInfo, AuthError, RefreshClaims, RefreshInfo, generate_access_token, generate_refresh_token};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set")).await.expect("postgres is not ready");
    let state = AppState { pool };

    let app = Router::new()
        .route("/", get(handler))
        .route("/authorize", post(authorize))
        .route("/protected", get(protected))
        .route("/refresh", post(refresh))
        .route("/users", get(users))
        .with_state(state);

    let port = std::env::var("PORT").expect("PORT must be set");
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
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

async fn users(State(AppState {pool}): State<AppState>) -> Result<String, AuthError> {
    let row: (String,) = sqlx::query_as("SELECT '홍길동'").fetch_one(&pool).await.map_err(|_| AuthError::WrongCredentials)?;
    Ok(row.0)
}