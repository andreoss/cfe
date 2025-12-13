use axum::{
    Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{App, html, theme::Theme};

const STYLESHEET: &str = include_str!("../static/style.css");
const LAST_PAGE: u32 = 10_000;

pub fn router(app: App) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/", get(index))
        .route("/sections/{slug}", get(section))
        .route("/static/style.css", get(stylesheet))
        .route("/theme", post(set_theme))
        .fallback(not_found)
        .with_state(app)
}

async fn health() -> &'static str {
    "ok"
}

async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLESHEET,
    )
}

async fn index(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    let theme = theme_of(&headers, &app);
    match app.sections().await {
        Ok(sections) => (
            StatusCode::OK,
            Html(html::page(
                theme,
                "Sections",
                &html::section_list(&sections),
            )),
        )
            .into_response(),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(html::message(
                theme,
                "The board is not answering",
                &error.to_string(),
            )),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct PageQuery {
    page: Option<String>,
}

async fn section(
    State(app): State<App>,
    Path(slug): Path<String>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let Some(board) = (match app.section(&slug).await {
        Ok(board) => board,
        Err(error) => return unavailable(theme, &error).into_response(),
    }) else {
        return (
            StatusCode::NOT_FOUND,
            Html(html::message(
                theme,
                "No such section",
                "This board has no section at that address.",
            )),
        )
            .into_response();
    };
    match app.topics(&slug, number).await {
        Ok(topics) => (
            StatusCode::OK,
            Html(html::page_at(
                theme,
                &board.title,
                &html::topic_list(&board, &topics),
                &section_address(&slug, number),
            )),
        )
            .into_response(),
        Err(error) => unavailable(theme, &error).into_response(),
    }
}

fn unavailable(theme: Theme, error: &client::ClientError) -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Html(html::message(
            theme,
            "The board is not answering",
            &error.to_string(),
        )),
    )
}

fn section_address(slug: &str, page: u32) -> String {
    if page <= 1 {
        format!("/sections/{}", client::encode_path(slug))
    } else {
        format!("/sections/{}?page={}", client::encode_path(slug), page)
    }
}

fn page_of(raw: Option<&str>) -> u32 {
    raw.and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|number| (1..=LAST_PAGE).contains(number))
        .unwrap_or(1)
}

#[derive(Deserialize)]
pub struct ThemeForm {
    theme: String,
    return_to: Option<String>,
}

async fn set_theme(State(app): State<App>, Form(form): Form<ThemeForm>) -> impl IntoResponse {
    let Some(theme) = Theme::parse(&form.theme) else {
        let known: Vec<&str> = Theme::ALL.iter().map(|theme| theme.name()).collect();
        return (
            StatusCode::BAD_REQUEST,
            Html(html::message(
                app.default_theme(),
                "No such theme",
                &format!("Choose one of: {}.", known.join(", ")),
            )),
        )
            .into_response();
    };
    let cookie = format!(
        "theme={}; Path=/; Max-Age=31536000; SameSite=Lax",
        theme.name()
    );
    (
        StatusCode::SEE_OTHER,
        [
            (header::SET_COOKIE, cookie),
            (header::LOCATION, return_to(form.return_to.as_deref())),
        ],
    )
        .into_response()
}

async fn not_found(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    let theme = theme_of(&headers, &app);
    (
        StatusCode::NOT_FOUND,
        Html(html::message(
            theme,
            "Nothing here",
            "This page does not exist.",
        )),
    )
        .into_response()
}

fn return_to(raw: Option<&str>) -> String {
    match raw {
        Some(value)
            if value.starts_with('/')
                && !value.starts_with("//")
                && !value.contains('\\')
                && !value.contains('\r')
                && !value.contains('\n') =>
        {
            value.to_owned()
        }
        _ => "/".to_owned(),
    }
}

fn theme_of(headers: &HeaderMap, app: &App) -> Theme {
    let cookies = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let chosen = cookies
        .split(';')
        .map(|pair| pair.trim())
        .find_map(|pair| pair.strip_prefix("theme="));
    app.theme_for(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_return_address_must_stay_on_this_site() {
        assert_eq!(return_to(Some("/sections/general")), "/sections/general");
        assert_eq!(return_to(Some("//elsewhere.example/x")), "/");
        assert_eq!(return_to(Some("https://elsewhere.example/")), "/");
        assert_eq!(return_to(Some("/x\\y")), "/");
        assert_eq!(return_to(None), "/");
    }
}
