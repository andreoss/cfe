use crate::handlers::AppState;
use crate::repository::PgUserRepository;
use crate::session_repository::PgSessionRepository;
use app::current_user as resolve_current_user;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use domain::{SessionToken, User};
use time::OffsetDateTime;

pub const SESSION_COOKIE: &str = "session";

pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token_str = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_owned())
            .ok_or((StatusCode::UNAUTHORIZED, "missing session"))?;
        let token = SessionToken::parse(&token_str)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid session"))?;
        let sessions = PgSessionRepository::new(state.pool.clone());
        let users = PgUserRepository::new(state.pool.clone());
        let user = resolve_current_user(&sessions, &users, &token, OffsetDateTime::now_utc())
            .await
            .ok_or((StatusCode::UNAUTHORIZED, "invalid session"))?;
        Ok(CurrentUser(user))
    }
}
