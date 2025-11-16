use crate::handlers::{AppState, ErrorResponse};
use axum::Json;
use crate::repository::PgUserRepository;
use crate::session_repository::PgSessionRepository;
use app::{active_ban, current_user as resolve_current_user};
use crate::enforcement_repository::PgEnforcementRepository;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use domain::{SessionToken, User};
use time::OffsetDateTime;

pub const SESSION_COOKIE: &str = "session";

pub struct CurrentUser(pub User);

pub struct OptionalUser(pub Option<User>);

fn unauthorized(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

async fn resolve(parts: &Parts, state: &AppState) -> Option<User> {
    let jar = CookieJar::from_headers(&parts.headers);
    let token_str = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned())?;
    let token = SessionToken::parse(&token_str).ok()?;
    let sessions = PgSessionRepository::new(state.pool.clone());
    let users = PgUserRepository::new(state.pool.clone());
    let now = OffsetDateTime::now_utc();
    let user = resolve_current_user(&sessions, &users, &token, now).await?;
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    match active_ban(&enforcement, user.id(), now).await {
        Some(_) => None,
        None => Some(user),
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        resolve(parts, state)
            .await
            .map(CurrentUser)
            .ok_or_else(|| unauthorized("missing session"))
    }
}

impl FromRequestParts<AppState> for OptionalUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(OptionalUser(resolve(parts, state).await))
    }
}
