use crate::auth::{CurrentUser, SESSION_COOKIE};
use crate::hasher::Argon2Hasher;
use crate::repository::PgUserRepository;
use crate::session_repository::PgSessionRepository;
use app::{
    RegisterError, SessionRepository, SignInError, create_session, register, sign_in,
    sign_out as end_session,
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use domain::{Email, Session, SessionId, SessionToken, UserId, Username};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn to_response(user: &domain::User) -> Json<UserResponse> {
    Json(UserResponse {
        id: user.id().as_uuid().to_string(),
        username: user.username().as_str().to_owned(),
    })
}

fn generate_token() -> SessionToken {
    let raw = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    SessionToken::parse(&raw).expect("generated token meets length requirement")
}

fn session_cookie(token: &SessionToken) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token.as_str().to_owned()))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .build()
}

async fn start_session(pool: &PgPool, user_id: UserId) -> SessionToken {
    let sessions = PgSessionRepository::new(pool.clone());
    let token = generate_token();
    let session = Session::new(
        SessionId::new(uuid::Uuid::new_v4()),
        user_id,
        token.clone(),
        OffsetDateTime::now_utc() + Duration::days(30),
    );
    create_session(&sessions, session).await;
    token
}

pub async fn register_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> Result<(CookieJar, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&body.username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let email = Email::parse(&body.email)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid email"))?;
    let repo = PgUserRepository::new(state.pool.clone());
    let hasher = Argon2Hasher;
    let id = UserId::new(uuid::Uuid::new_v4());
    let user = register(&repo, &hasher, id, username, email, &body.password)
        .await
        .map_err(|e| match e {
            RegisterError::UsernameTaken => error(StatusCode::CONFLICT, "username taken"),
            RegisterError::EmailTaken => error(StatusCode::CONFLICT, "email taken"),
        })?;
    let token = start_session(&state.pool, user.id()).await;
    Ok((jar.add(session_cookie(&token)), to_response(&user)))
}

pub async fn sign_in_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<SignInRequest>,
) -> Result<(CookieJar, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&body.username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let repo = PgUserRepository::new(state.pool.clone());
    let hasher = Argon2Hasher;
    let user = sign_in(&repo, &hasher, &username, &body.password)
        .await
        .map_err(|e| match e {
            SignInError::NotFound => error(StatusCode::UNAUTHORIZED, "invalid credentials"),
            SignInError::WrongPassword => error(StatusCode::UNAUTHORIZED, "invalid credentials"),
        })?;
    let token = start_session(&state.pool, user.id()).await;
    Ok((jar.add(session_cookie(&token)), to_response(&user)))
}

pub async fn me_handler(CurrentUser(user): CurrentUser) -> Json<UserResponse> {
    to_response(&user)
}

pub async fn sign_out_handler(State(state): State<AppState>, jar: CookieJar) -> CookieJar {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        if let Ok(token) = SessionToken::parse(cookie.value()) {
            let sessions = PgSessionRepository::new(state.pool.clone());
            if let Some(session) = sessions.find_by_token(&token).await {
                end_session(&sessions, session.id()).await;
            }
        }
    }
    let removal = Cookie::build((SESSION_COOKIE, "")).path("/").build();
    jar.remove(removal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_meet_minimum_length_and_are_distinct() {
        let a = generate_token();
        let b = generate_token();
        assert!(a.as_str().len() >= 32);
        assert_ne!(a, b);
    }

    #[test]
    fn session_cookie_is_http_only_and_strict() {
        let token = SessionToken::parse("0123456789abcdef").unwrap();
        let cookie = session_cookie(&token);
        assert_eq!(cookie.name(), SESSION_COOKIE);
        assert_eq!(cookie.value(), token.as_str());
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(cookie.path(), Some("/"));
    }
}
