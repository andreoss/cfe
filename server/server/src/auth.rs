use crate::handlers::{AppState, ErrorResponse};
use app::{active_ban, current_user as resolve_current_user};
use axum::Json;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use domain::{Address, ClientString, SessionToken, User};
use std::net::SocketAddr;
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
    let sessions = state.backend.sessions();
    let users = state.backend.users();
    let now = OffsetDateTime::now_utc();
    let user = resolve_current_user(&*sessions, &*users, &token, now).await?;
    let enforcement = state.backend.enforcement();
    match active_ban(&*enforcement, user.id(), now).await {
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

pub struct ClientIp(pub Option<Address>);

fn forwarded_address(parts: &Parts) -> Option<Address> {
    if let Some(value) = parts
        .headers
        .get("x-forwarded-for")
        .or_else(|| parts.headers.get("x-real-ip"))
    {
        let first = value.to_str().ok()?.split(',').next()?.trim();
        if let Ok(addr) = Address::parse(first) {
            return Some(addr);
        }
    }
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .and_then(|ip| Address::parse(&ip).ok())
}

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(forwarded_address(parts)))
    }
}

pub struct ClientAgent(pub Option<ClientString>);

fn declared_client(parts: &Parts) -> Option<ClientString> {
    let value = parts.headers.get(axum::http::header::USER_AGENT)?;
    ClientString::parse(value.to_str().ok()?).ok()
}

impl FromRequestParts<AppState> for ClientAgent {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(ClientAgent(declared_client(parts)))
    }
}
