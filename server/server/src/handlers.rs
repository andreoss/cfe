use crate::hasher::Argon2Hasher;
use crate::repository::PgUserRepository;
use app::{RegisterError, SignInError, register, sign_in};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use domain::{Email, UserId, Username};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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

pub async fn register_handler(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&body.username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let email = Email::parse(&body.email)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid email"))?;
    let repo = PgUserRepository::new(state.pool);
    let hasher = Argon2Hasher;
    let id = UserId::new(uuid::Uuid::new_v4());
    let user = register(&repo, &hasher, id, username, email, &body.password)
        .await
        .map_err(|e| match e {
            RegisterError::UsernameTaken => error(StatusCode::CONFLICT, "username taken"),
            RegisterError::EmailTaken => error(StatusCode::CONFLICT, "email taken"),
        })?;
    Ok(Json(UserResponse {
        id: user.id().as_uuid().to_string(),
        username: user.username().as_str().to_owned(),
    }))
}

pub async fn sign_in_handler(
    State(state): State<AppState>,
    Json(body): Json<SignInRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&body.username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let repo = PgUserRepository::new(state.pool);
    let hasher = Argon2Hasher;
    let user = sign_in(&repo, &hasher, &username, &body.password)
        .await
        .map_err(|e| match e {
            SignInError::NotFound => error(StatusCode::UNAUTHORIZED, "invalid credentials"),
            SignInError::WrongPassword => error(StatusCode::UNAUTHORIZED, "invalid credentials"),
        })?;
    Ok(Json(UserResponse {
        id: user.id().as_uuid().to_string(),
        username: user.username().as_str().to_owned(),
    }))
}
