mod handlers;
mod hasher;
mod repository;

use axum::Router;
use axum::routing::post;
use handlers::{AppState, register_handler, sign_in_handler};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    let state = AppState { pool };
    let app = Router::new()
        .route("/api/register", post(register_handler))
        .route("/api/sign-in", post(sign_in_handler))
        .with_state(state);
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("bind listener");
    axum::serve(listener, app).await.expect("serve");
}
