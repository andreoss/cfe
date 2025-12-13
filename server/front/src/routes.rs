use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};

use crate::{App, html};

pub fn router(app: App) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/", get(index))
        .fallback(not_found)
        .with_state(app)
}

async fn health() -> &'static str {
    "ok"
}

async fn index(State(app): State<App>) -> impl IntoResponse {
    match app.sections().await {
        Ok(sections) => (
            StatusCode::OK,
            Html(html::page(
                app.default_theme(),
                "Sections",
                &html::section_list(&sections),
            )),
        )
            .into_response(),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(html::message(
                app.default_theme(),
                "The board is not answering",
                &error.to_string(),
            )),
        )
            .into_response(),
    }
}

async fn not_found(State(app): State<App>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(html::message(
            app.default_theme(),
            "Nothing here",
            "This page does not exist.",
        )),
    )
        .into_response()
}
