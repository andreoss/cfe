use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};

use crate::App;

pub fn router(app: App) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .fallback(not_found)
        .with_state(app)
}

async fn health() -> &'static str {
    "ok"
}

async fn not_found(State(_app): State<App>) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Nothing here.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_health_route_says_it_is_there() {
        assert_eq!(health().await, "ok");
    }

    #[tokio::test]
    async fn an_unknown_address_is_refused() {
        let response = not_found(State(crate::App::new(&crate::config::Config::from_vars(
            &std::collections::BTreeMap::new(),
        ))))
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
