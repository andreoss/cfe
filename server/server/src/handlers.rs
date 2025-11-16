use crate::auth::{CurrentUser, OptionalUser, SESSION_COOKIE};
use crate::avatar_repository::PgAvatarRepository;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use crate::bookmark_repository::PgBookmarkRepository;
use crate::comment_repository::PgCommentRepository;
use crate::enforcement_repository::PgEnforcementRepository;
use crate::hasher::Argon2Hasher;
use crate::reaction_repository::PgReactionRepository;
use crate::repository::PgUserRepository;
use crate::notification_repository::PgNotificationRepository;
use crate::poll_repository::PgPollRepository;
use crate::search_repository::PgSearchRepository;
use crate::section_repository::PgSectionRepository;
use crate::session_repository::PgSessionRepository;
use crate::topic_repository::PgTopicRepository;
use app::{
    AvatarLookupError, BookmarkError, ChangePasswordError, CommentRepository, CreatePollError,
    EnforcementError, acknowledge_warnings, active_ban, ban_user, ignore_user, ignored_by,
    lift_ban, list_warnings, stop_ignoring, warn_user,
    CreateTopicError, DeleteError, EditError, change_password, clear_avatar, deregister,
    get_avatar, set_avatar,
    ListTopicsError, MarkReadError, PollResults, VoteError, cast_vote, create_poll, poll_results,
    PostCommentError, RegisterError, SectionRepository, SessionRepository, SignInError,
    TopicRepository, UpdateBioError, UserRepository, add_bookmark, count_unread, create_session,
    create_topic, delete_comment, delete_topic, edit_comment, edit_topic, get_topic,
    ReactionSummary, clear_reaction, is_bookmarked, list_bookmarked_topics, list_comments,
    list_notifications, list_sections, list_topics, list_topics_by_tag, mark_read, post_comment,
    react, register, remove_bookmark, search, sign_in, sign_out as end_session,
    summarize_reactions, update_bio,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use domain::{
    Avatar, AvatarError, Bio, Body, CommentId, Email, Password, PollId, PollOption, PollOptionId,
    Question, Reason,
    ReactionTarget, SearchHit, Session, SessionId, SessionToken, Slug, TagSet, Title, TopicId,
    UserId, Username,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::format_description::well_known::Rfc3339;
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
    pub role: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct UpdateBioRequest {
    pub bio: Option<String>,
}

#[derive(Serialize)]
pub struct ProfileResponse {
    pub id: String,
    pub username: String,
    pub bio: Option<String>,
}

#[derive(Serialize)]
pub struct SectionResponse {
    pub slug: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct CreateTopicRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct TopicResponse {
    pub id: String,
    pub section_slug: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub author_username: String,
    pub created_at: String,
    pub deleted: bool,
    pub deleted_reason: Option<String>,
    pub edited: bool,
}

#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub body: String,
    pub parent_id: Option<String>,
}

#[derive(Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub topic_id: String,
    pub parent_id: Option<String>,
    pub body: String,
    pub author_username: String,
    pub created_at: String,
    pub deleted: bool,
    pub deleted_reason: Option<String>,
    pub edited: bool,
    pub ignored: bool,
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub reason: String,
}

#[derive(Deserialize)]
pub struct EditTopicRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct EditCommentRequest {
    pub body: String,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchHitResponse {
    Topic(TopicResponse),
    Comment(CommentResponse),
}

fn error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
}

fn edit_error(e: EditError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        EditError::NotFound => error(StatusCode::NOT_FOUND, "not found"),
        EditError::NotAuthorized => error(StatusCode::FORBIDDEN, "not the author"),
        EditError::Deleted => error(StatusCode::CONFLICT, "removed content cannot be edited"),
    }
}

fn role_str(role: domain::Role) -> &'static str {
    match role {
        domain::Role::User => "user",
        domain::Role::Moderator => "moderator",
    }
}

fn to_response(user: &domain::User) -> Json<UserResponse> {
    Json(UserResponse {
        id: user.id().as_uuid().to_string(),
        username: user.username().as_str().to_owned(),
        role: role_str(user.role()).to_owned(),
    })
}

fn to_profile_response(user: &domain::User) -> Json<ProfileResponse> {
    Json(ProfileResponse {
        id: user.id().as_uuid().to_string(),
        username: user.username().as_str().to_owned(),
        bio: user.bio().map(|b| b.as_str().to_owned()),
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
    Password::parse(&body.password)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "password too short"))?;
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
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    if let Some(ban) = active_ban(&enforcement, user.id(), OffsetDateTime::now_utc()).await {
        return Err(error(
            StatusCode::FORBIDDEN,
            &format!("account suspended: {}", ban.reason().as_str()),
        ));
    }
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

pub async fn get_profile_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let repo = PgUserRepository::new(state.pool);
    let user = repo
        .find_by_username(&username)
        .await
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "user not found"))?;
    Ok(to_profile_response(&user))
}

pub async fn update_bio_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<UpdateBioRequest>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let bio = Bio::parse(body.bio.as_deref().unwrap_or(""))
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "bio too long"))?;
    let repo = PgUserRepository::new(state.pool);
    let updated = update_bio(&repo, current.id(), bio)
        .await
        .map_err(|e| match e {
            UpdateBioError::NotFound => error(StatusCode::NOT_FOUND, "user not found"),
        })?;
    Ok(to_profile_response(&updated))
}

async fn topic_response(
    pool: &PgPool,
    topic: &domain::Topic,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let sections = PgSectionRepository::new(pool.clone());
    let users = PgUserRepository::new(pool.clone());
    let section = sections
        .find_by_id(topic.section_id())
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "section missing"))?;
    let author = users
        .find_by_id(topic.author_id())
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "author missing"))?;
    let created_at = topic
        .created_at()
        .format(&Rfc3339)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?;
    Ok(Json(TopicResponse {
        id: topic.id().as_uuid().to_string(),
        section_slug: section.slug().as_str().to_owned(),
        title: topic.title().as_str().to_owned(),
        body: topic.body().as_str().to_owned(),
        tags: topic
            .tags()
            .as_slice()
            .iter()
            .map(|t| t.as_str().to_owned())
            .collect(),
        author_username: author.username().as_str().to_owned(),
        created_at,
        deleted: topic.is_deleted(),
        deleted_reason: topic.deletion().map(|d| d.reason().as_str().to_owned()),
        edited: topic.is_edited(),
    }))
}

pub async fn list_sections_handler(State(state): State<AppState>) -> Json<Vec<SectionResponse>> {
    let repo = PgSectionRepository::new(state.pool);
    let sections = list_sections(&repo).await;
    Json(
        sections
            .into_iter()
            .map(|s| SectionResponse {
                slug: s.slug().as_str().to_owned(),
                title: s.title().as_str().to_owned(),
            })
            .collect(),
    )
}

pub async fn list_topics_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<TopicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let slug = Slug::parse(&slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section slug"))?;
    let sections = PgSectionRepository::new(state.pool.clone());
    let topics = PgTopicRepository::new(state.pool.clone());
    let list = list_topics(&sections, &topics, &slug)
        .await
        .map_err(|e| match e {
            ListTopicsError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
        })?;
    let mut responses = Vec::with_capacity(list.len());
    for topic in &list {
        responses.push(topic_response(&state.pool, topic).await?.0);
    }
    Ok(Json(responses))
}

pub async fn create_topic_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<CreateTopicRequest>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let slug = Slug::parse(&slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section slug"))?;
    let title = Title::parse(&body.title)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid title"))?;
    let topic_body = Body::parse(&body.body)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid body"))?;
    let tags = TagSet::parse(&body.tags)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid tags"))?;
    let sections = PgSectionRepository::new(state.pool.clone());
    let topics = PgTopicRepository::new(state.pool.clone());
    let topic = create_topic(
        &sections,
        &topics,
        domain::TopicId::new(uuid::Uuid::new_v4()),
        &slug,
        current.id(),
        title,
        topic_body,
        tags,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        CreateTopicError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
    })?;
    topic_response(&state.pool, &topic).await
}

pub async fn get_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let topics = PgTopicRepository::new(state.pool.clone());
    let topic = get_topic(&topics, domain::TopicId::new(id))
        .await
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "topic not found"))?;
    topic_response(&state.pool, &topic).await
}

pub async fn list_topics_by_tag_handler(
    State(state): State<AppState>,
    Path(tag): Path<String>,
) -> Result<Json<Vec<TopicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let tag =
        Slug::parse(&tag).map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid tag"))?;
    let topics = PgTopicRepository::new(state.pool.clone());
    let list = list_topics_by_tag(&topics, &tag).await;
    let mut responses = Vec::with_capacity(list.len());
    for topic in &list {
        responses.push(topic_response(&state.pool, topic).await?.0);
    }
    Ok(Json(responses))
}

async fn comment_response(
    pool: &PgPool,
    comment: &domain::Comment,
) -> Result<CommentResponse, (StatusCode, Json<ErrorResponse>)> {
    let users = PgUserRepository::new(pool.clone());
    let author = users
        .find_by_id(comment.author_id())
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "author missing"))?;
    let created_at = comment
        .created_at()
        .format(&Rfc3339)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?;
    Ok(CommentResponse {
        id: comment.id().as_uuid().to_string(),
        topic_id: comment.topic_id().as_uuid().to_string(),
        parent_id: comment.parent_id().map(|p| p.as_uuid().to_string()),
        body: comment.body().as_str().to_owned(),
        author_username: author.username().as_str().to_owned(),
        created_at,
        deleted: comment.is_deleted(),
        deleted_reason: comment.deletion().map(|d| d.reason().as_str().to_owned()),
        edited: comment.is_edited(),
        ignored: false,
    })
}

pub async fn list_comments_handler(
    State(state): State<AppState>,
    Path(topic_id): Path<uuid::Uuid>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<Vec<CommentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let comments = PgCommentRepository::new(state.pool.clone());
    let list = list_comments(&comments, TopicId::new(topic_id)).await;
    let ignored = match &viewer {
        Some(user) => {
            let enforcement = PgEnforcementRepository::new(state.pool.clone());
            ignored_by(&enforcement, user.id()).await
        }
        None => Vec::new(),
    };
    let mut responses = Vec::with_capacity(list.len());
    for comment in &list {
        let mut response = comment_response(&state.pool, comment).await?;
        if ignored.contains(&comment.author_id()) {
            response.body = String::new();
            response.ignored = true;
        }
        responses.push(response);
    }
    Ok(Json(responses))
}

pub async fn post_comment_handler(
    State(state): State<AppState>,
    Path(topic_id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<CreateCommentRequest>,
) -> Result<Json<CommentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let comment_body = Body::parse(&body.body)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid body"))?;
    let parent_id = body
        .parent_id
        .map(|raw| {
            uuid::Uuid::parse_str(&raw)
                .map(CommentId::new)
                .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid parent_id"))
        })
        .transpose()?;
    let topics = PgTopicRepository::new(state.pool.clone());
    let comments = PgCommentRepository::new(state.pool.clone());
    let notifications = PgNotificationRepository::new(state.pool.clone());
    let comment = post_comment(
        &topics,
        &comments,
        &notifications,
        CommentId::new(uuid::Uuid::new_v4()),
        domain::NotificationId::new(uuid::Uuid::new_v4()),
        TopicId::new(topic_id),
        current.id(),
        parent_id,
        comment_body,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        PostCommentError::TopicNotFound => error(StatusCode::NOT_FOUND, "topic not found"),
        PostCommentError::ParentNotFound => error(StatusCode::NOT_FOUND, "parent not found"),
        PostCommentError::ParentInDifferentTopic => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "parent in different topic",
        ),
    })?;
    Ok(Json(comment_response(&state.pool, &comment).await?))
}

pub async fn delete_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<DeleteRequest>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let topics = PgTopicRepository::new(state.pool.clone());
    let topic = delete_topic(
        &topics,
        &current,
        TopicId::new(id),
        reason,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        DeleteError::NotFound => error(StatusCode::NOT_FOUND, "topic not found"),
        DeleteError::NotAuthorized => error(StatusCode::FORBIDDEN, "moderator role required"),
    })?;
    topic_response(&state.pool, &topic).await
}

pub async fn delete_comment_handler(
    State(state): State<AppState>,
    Path((_topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<DeleteRequest>,
) -> Result<Json<CommentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let comments = PgCommentRepository::new(state.pool.clone());
    let comment = delete_comment(
        &comments,
        &current,
        CommentId::new(id),
        reason,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        DeleteError::NotFound => error(StatusCode::NOT_FOUND, "comment not found"),
        DeleteError::NotAuthorized => error(StatusCode::FORBIDDEN, "moderator role required"),
    })?;
    Ok(Json(comment_response(&state.pool, &comment).await?))
}

pub async fn edit_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<EditTopicRequest>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let title = Title::parse(&body.title)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid title"))?;
    let topic_body = Body::parse(&body.body)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid body"))?;
    let tags = TagSet::parse(&body.tags)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid tags"))?;
    let topics = PgTopicRepository::new(state.pool.clone());
    let topic = edit_topic(
        &topics,
        &current,
        TopicId::new(id),
        title,
        topic_body,
        tags,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(edit_error)?;
    topic_response(&state.pool, &topic).await
}

pub async fn edit_comment_handler(
    State(state): State<AppState>,
    Path((_topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<EditCommentRequest>,
) -> Result<Json<CommentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let comment_body = Body::parse(&body.body)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid body"))?;
    let comments = PgCommentRepository::new(state.pool.clone());
    let comment = edit_comment(
        &comments,
        &current,
        CommentId::new(id),
        comment_body,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(edit_error)?;
    Ok(Json(comment_response(&state.pool, &comment).await?))
}

#[derive(Serialize)]
pub struct NotificationResponse {
    pub id: String,
    pub topic_id: String,
    pub topic_title: String,
    pub comment_id: String,
    pub actor_username: String,
    pub created_at: String,
    pub read: bool,
}

#[derive(Serialize)]
pub struct UnreadCountResponse {
    pub unread: u64,
}

async fn notification_response(
    pool: &PgPool,
    notification: &domain::Notification,
) -> Result<NotificationResponse, (StatusCode, Json<ErrorResponse>)> {
    let users = PgUserRepository::new(pool.clone());
    let topics = PgTopicRepository::new(pool.clone());
    let actor = users
        .find_by_id(notification.actor_id())
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "actor missing"))?;
    let topic = topics
        .find_by_id(notification.topic_id())
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "topic missing"))?;
    let created_at = notification
        .created_at()
        .format(&Rfc3339)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?;
    Ok(NotificationResponse {
        id: notification.id().as_uuid().to_string(),
        topic_id: notification.topic_id().as_uuid().to_string(),
        topic_title: topic.title().as_str().to_owned(),
        comment_id: notification.comment_id().as_uuid().to_string(),
        actor_username: actor.username().as_str().to_owned(),
        created_at,
        read: notification.is_read(),
    })
}

pub async fn list_notifications_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<Vec<NotificationResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = PgNotificationRepository::new(state.pool.clone());
    let list = list_notifications(&repo, current.id()).await;
    let mut responses = Vec::with_capacity(list.len());
    for notification in &list {
        responses.push(notification_response(&state.pool, notification).await?);
    }
    Ok(Json(responses))
}

pub async fn unread_count_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Json<UnreadCountResponse> {
    let repo = PgNotificationRepository::new(state.pool.clone());
    Json(UnreadCountResponse {
        unread: count_unread(&repo, current.id()).await,
    })
}

pub async fn mark_notification_read_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = PgNotificationRepository::new(state.pool.clone());
    let notification = mark_read(
        &repo,
        current.id(),
        domain::NotificationId::new(id),
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        MarkReadError::NotFound => error(StatusCode::NOT_FOUND, "notification not found"),
    })?;
    Ok(Json(notification_response(&state.pool, &notification).await?))
}

#[derive(Serialize)]
pub struct BookmarkStateResponse {
    pub bookmarked: bool,
}

pub async fn add_bookmark_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<BookmarkStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let topics = PgTopicRepository::new(state.pool.clone());
    let bookmarks = PgBookmarkRepository::new(state.pool.clone());
    add_bookmark(
        &topics,
        &bookmarks,
        current.id(),
        TopicId::new(id),
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        BookmarkError::TopicNotFound => error(StatusCode::NOT_FOUND, "topic not found"),
    })?;
    Ok(Json(BookmarkStateResponse { bookmarked: true }))
}

pub async fn remove_bookmark_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Json<BookmarkStateResponse> {
    let bookmarks = PgBookmarkRepository::new(state.pool.clone());
    remove_bookmark(&bookmarks, current.id(), TopicId::new(id)).await;
    Json(BookmarkStateResponse { bookmarked: false })
}

pub async fn bookmark_state_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Json<BookmarkStateResponse> {
    let bookmarks = PgBookmarkRepository::new(state.pool.clone());
    Json(BookmarkStateResponse {
        bookmarked: is_bookmarked(&bookmarks, current.id(), TopicId::new(id)).await,
    })
}

pub async fn list_bookmarks_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<Vec<TopicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let bookmarks = PgBookmarkRepository::new(state.pool.clone());
    let list = list_bookmarked_topics(&bookmarks, current.id()).await;
    let mut responses = Vec::with_capacity(list.len());
    for topic in &list {
        responses.push(topic_response(&state.pool, topic).await?.0);
    }
    Ok(Json(responses))
}

#[derive(Deserialize)]
pub struct ReactRequest {
    pub kind: String,
}

#[derive(Serialize)]
pub struct ReactionCountResponse {
    pub kind: String,
    pub count: u64,
}

#[derive(Serialize)]
pub struct ReactionsResponse {
    pub counts: Vec<ReactionCountResponse>,
    pub mine: Option<String>,
}

fn to_reactions_response(summary: ReactionSummary) -> ReactionsResponse {
    ReactionsResponse {
        counts: summary
            .counts
            .into_iter()
            .map(|(kind, count)| ReactionCountResponse {
                kind: kind.as_str().to_owned(),
                count,
            })
            .collect(),
        mine: summary.mine.map(|k| k.as_str().to_owned()),
    }
}

async fn reaction_target(
    pool: &PgPool,
    topic_id: uuid::Uuid,
    comment_id: Option<uuid::Uuid>,
) -> Result<ReactionTarget, (StatusCode, Json<ErrorResponse>)> {
    match comment_id {
        Some(id) => {
            let comments = PgCommentRepository::new(pool.clone());
            let comment = comments
                .find_by_id(CommentId::new(id))
                .await
                .ok_or_else(|| error(StatusCode::NOT_FOUND, "comment not found"))?;
            if comment.topic_id() != TopicId::new(topic_id) {
                return Err(error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "comment in different topic",
                ));
            }
            Ok(ReactionTarget::Comment(comment.id()))
        }
        None => {
            let topics = PgTopicRepository::new(pool.clone());
            let topic = topics
                .find_by_id(TopicId::new(topic_id))
                .await
                .ok_or_else(|| error(StatusCode::NOT_FOUND, "topic not found"))?;
            Ok(ReactionTarget::Topic(topic.id()))
        }
    }
}

async fn reactions_for(
    pool: &PgPool,
    viewer: Option<&domain::User>,
    target: ReactionTarget,
) -> Json<ReactionsResponse> {
    let repo = PgReactionRepository::new(pool.clone());
    let summary = summarize_reactions(&repo, viewer.map(|u| u.id()), target).await;
    Json(to_reactions_response(summary))
}

pub async fn topic_reactions_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = reaction_target(&state.pool, id, None).await?;
    Ok(reactions_for(&state.pool, viewer.as_ref(), target).await)
}

pub async fn react_to_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ReactRequest>,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let kind = domain::ReactionKind::parse(&body.kind)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "unknown reaction"))?;
    let target = reaction_target(&state.pool, id, None).await?;
    let repo = PgReactionRepository::new(state.pool.clone());
    react(&repo, current.id(), target, kind, OffsetDateTime::now_utc()).await;
    Ok(reactions_for(&state.pool, Some(&current), target).await)
}

pub async fn clear_topic_reaction_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = reaction_target(&state.pool, id, None).await?;
    let repo = PgReactionRepository::new(state.pool.clone());
    clear_reaction(&repo, current.id(), target).await;
    Ok(reactions_for(&state.pool, Some(&current), target).await)
}

pub async fn comment_reactions_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = reaction_target(&state.pool, topic_id, Some(id)).await?;
    Ok(reactions_for(&state.pool, viewer.as_ref(), target).await)
}

pub async fn react_to_comment_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ReactRequest>,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let kind = domain::ReactionKind::parse(&body.kind)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "unknown reaction"))?;
    let target = reaction_target(&state.pool, topic_id, Some(id)).await?;
    let repo = PgReactionRepository::new(state.pool.clone());
    react(&repo, current.id(), target, kind, OffsetDateTime::now_utc()).await;
    Ok(reactions_for(&state.pool, Some(&current), target).await)
}

pub async fn clear_comment_reaction_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = reaction_target(&state.pool, topic_id, Some(id)).await?;
    let repo = PgReactionRepository::new(state.pool.clone());
    clear_reaction(&repo, current.id(), target).await;
    Ok(reactions_for(&state.pool, Some(&current), target).await)
}

#[derive(Deserialize)]
pub struct CreatePollRequest {
    pub question: String,
    pub options: Vec<String>,
}

#[derive(Deserialize)]
pub struct VoteRequest {
    pub option_id: String,
}

#[derive(Serialize)]
pub struct PollOptionResponse {
    pub id: String,
    pub text: String,
    pub votes: u64,
}

#[derive(Serialize)]
pub struct PollResponse {
    pub id: String,
    pub topic_id: String,
    pub question: String,
    pub options: Vec<PollOptionResponse>,
    pub mine: Option<String>,
    pub total_votes: u64,
}

fn to_poll_response(results: PollResults) -> PollResponse {
    let options: Vec<PollOptionResponse> = results
        .poll
        .options()
        .iter()
        .map(|o| PollOptionResponse {
            id: o.id().as_uuid().to_string(),
            text: o.text().as_str().to_owned(),
            votes: results
                .counts
                .iter()
                .find(|(id, _)| *id == o.id())
                .map(|(_, c)| *c)
                .unwrap_or(0),
        })
        .collect();
    PollResponse {
        id: results.poll.id().as_uuid().to_string(),
        topic_id: results.poll.topic_id().as_uuid().to_string(),
        question: results.poll.question().as_str().to_owned(),
        total_votes: options.iter().map(|o| o.votes).sum(),
        options,
        mine: results.mine.map(|id| id.as_uuid().to_string()),
    }
}

pub async fn get_poll_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<PollResponse>, (StatusCode, Json<ErrorResponse>)> {
    let polls = PgPollRepository::new(state.pool.clone());
    let results = poll_results(&polls, viewer.map(|u| u.id()), TopicId::new(id))
        .await
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "poll not found"))?;
    Ok(Json(to_poll_response(results)))
}

pub async fn create_poll_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<CreatePollRequest>,
) -> Result<Json<PollResponse>, (StatusCode, Json<ErrorResponse>)> {
    let question = Question::parse(&body.question)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid question"))?;
    let mut options = Vec::with_capacity(body.options.len());
    for text in &body.options {
        let parsed = Question::parse(text)
            .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid option"))?;
        options.push(PollOption::new(
            PollOptionId::new(uuid::Uuid::new_v4()),
            parsed,
        ));
    }
    let topics = PgTopicRepository::new(state.pool.clone());
    let polls = PgPollRepository::new(state.pool.clone());
    create_poll(
        &topics,
        &polls,
        &current,
        PollId::new(uuid::Uuid::new_v4()),
        TopicId::new(id),
        question,
        options,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        CreatePollError::TopicNotFound => error(StatusCode::NOT_FOUND, "topic not found"),
        CreatePollError::NotTheAuthor => error(StatusCode::FORBIDDEN, "not the author"),
        CreatePollError::AlreadyExists => error(StatusCode::CONFLICT, "poll already exists"),
        CreatePollError::Invalid(_) => {
            error(StatusCode::UNPROCESSABLE_ENTITY, "invalid option count")
        }
    })?;
    let results = poll_results(&polls, Some(current.id()), TopicId::new(id))
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "poll missing"))?;
    Ok(Json(to_poll_response(results)))
}

pub async fn vote_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<VoteRequest>,
) -> Result<Json<PollResponse>, (StatusCode, Json<ErrorResponse>)> {
    let option_id = uuid::Uuid::parse_str(&body.option_id)
        .map(PollOptionId::new)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid option id"))?;
    let polls = PgPollRepository::new(state.pool.clone());
    cast_vote(
        &polls,
        &current,
        TopicId::new(id),
        option_id,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        VoteError::PollNotFound => error(StatusCode::NOT_FOUND, "poll not found"),
        VoteError::UnknownOption => error(StatusCode::UNPROCESSABLE_ENTITY, "unknown option"),
    })?;
    let results = poll_results(&polls, Some(current.id()), TopicId::new(id))
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "poll missing"))?;
    Ok(Json(to_poll_response(results)))
}

#[derive(Deserialize)]
pub struct BanRequest {
    pub reason: String,
    pub days: Option<i64>,
}

#[derive(Deserialize)]
pub struct WarnRequest {
    pub reason: String,
}

#[derive(Serialize)]
pub struct BanResponse {
    pub reason: String,
    pub until: Option<String>,
}

#[derive(Serialize)]
pub struct WarningResponse {
    pub id: String,
    pub reason: String,
    pub created_at: String,
    pub acknowledged: bool,
}

fn enforcement_error(e: EnforcementError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        EnforcementError::NotAuthorized => {
            error(StatusCode::FORBIDDEN, "moderator role required")
        }
        EnforcementError::UserNotFound => error(StatusCode::NOT_FOUND, "user not found"),
        EnforcementError::NotYourself => {
            error(StatusCode::UNPROCESSABLE_ENTITY, "not yourself")
        }
    }
}

async fn find_user_id(
    pool: &PgPool,
    username: &str,
) -> Result<UserId, (StatusCode, Json<ErrorResponse>)> {
    let parsed = Username::parse(username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let users = PgUserRepository::new(pool.clone());
    users
        .find_by_username(&parsed)
        .await
        .map(|u| u.id())
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "user not found"))
}

pub async fn ban_user_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<BanRequest>,
) -> Result<Json<BanResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let target = find_user_id(&state.pool, &username).await?;
    let now = OffsetDateTime::now_utc();
    let until = body.days.map(|d| now + Duration::days(d));
    let users = PgUserRepository::new(state.pool.clone());
    let sessions = PgSessionRepository::new(state.pool.clone());
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    let ban = ban_user(
        &users,
        &sessions,
        &enforcement,
        &current,
        target,
        reason,
        now,
        until,
    )
    .await
    .map_err(enforcement_error)?;
    Ok(Json(BanResponse {
        reason: ban.reason().as_str().to_owned(),
        until: ban.until().and_then(|u| u.format(&Rfc3339).ok()),
    }))
}

pub async fn lift_ban_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state.pool, &username).await?;
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    lift_ban(&enforcement, &current, target)
        .await
        .map_err(enforcement_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn warn_user_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<WarnRequest>,
) -> Result<Json<WarningResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let target = find_user_id(&state.pool, &username).await?;
    let users = PgUserRepository::new(state.pool.clone());
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    let warning = warn_user(
        &users,
        &enforcement,
        &current,
        domain::WarningId::new(uuid::Uuid::new_v4()),
        target,
        reason,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(enforcement_error)?;
    Ok(Json(WarningResponse {
        id: warning.id().as_uuid().to_string(),
        reason: warning.reason().as_str().to_owned(),
        created_at: warning
            .created_at()
            .format(&Rfc3339)
            .unwrap_or_else(|_| String::new()),
        acknowledged: warning.is_acknowledged(),
    }))
}

pub async fn my_warnings_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Json<Vec<WarningResponse>> {
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    Json(
        list_warnings(&enforcement, current.id())
            .await
            .into_iter()
            .map(|w| WarningResponse {
                id: w.id().as_uuid().to_string(),
                reason: w.reason().as_str().to_owned(),
                created_at: w.created_at().format(&Rfc3339).unwrap_or_else(|_| String::new()),
                acknowledged: w.is_acknowledged(),
            })
            .collect(),
    )
}

pub async fn acknowledge_warnings_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> StatusCode {
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    acknowledge_warnings(&enforcement, current.id()).await;
    StatusCode::NO_CONTENT
}

#[derive(Serialize)]
pub struct IgnoreStateResponse {
    pub ignored: bool,
}

pub async fn ignore_user_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<IgnoreStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state.pool, &username).await?;
    let users = PgUserRepository::new(state.pool.clone());
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    ignore_user(&users, &enforcement, &current, target)
        .await
        .map_err(enforcement_error)?;
    Ok(Json(IgnoreStateResponse { ignored: true }))
}

pub async fn stop_ignoring_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<IgnoreStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state.pool, &username).await?;
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    stop_ignoring(&enforcement, &current, target).await;
    Ok(Json(IgnoreStateResponse { ignored: false }))
}

pub async fn ignore_state_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<IgnoreStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state.pool, &username).await?;
    let enforcement = PgEnforcementRepository::new(state.pool.clone());
    Ok(Json(IgnoreStateResponse {
        ignored: ignored_by(&enforcement, current.id()).await.contains(&target),
    }))
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<(CookieJar, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    let new_password = Password::parse(&body.new_password)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid new password"))?;
    let users = PgUserRepository::new(state.pool.clone());
    let sessions = PgSessionRepository::new(state.pool.clone());
    let updated = change_password(
        &users,
        &sessions,
        &Argon2Hasher,
        &current,
        &body.current_password,
        new_password,
    )
    .await
    .map_err(|e| match e {
        ChangePasswordError::WrongPassword => {
            error(StatusCode::UNAUTHORIZED, "wrong current password")
        }
    })?;
    let token = start_session(&state.pool, updated.id()).await;
    Ok((jar.add(session_cookie(&token)), to_response(&updated)))
}

pub async fn deregister_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    CurrentUser(current): CurrentUser,
) -> (CookieJar, Json<UserResponse>) {
    let users = PgUserRepository::new(state.pool.clone());
    let sessions = PgSessionRepository::new(state.pool.clone());
    let avatars = PgAvatarRepository::new(state.pool.clone());
    clear_avatar(&avatars, current.id()).await;
    let gone = deregister(&users, &sessions, &current, OffsetDateTime::now_utc()).await;
    (jar.remove(Cookie::from(SESSION_COOKIE)), to_response(&gone))
}

#[derive(Deserialize)]
pub struct UploadAvatarRequest {
    pub data: String,
}

#[derive(Serialize)]
pub struct AvatarStateResponse {
    pub has_avatar: bool,
}

pub async fn upload_avatar_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<UploadAvatarRequest>,
) -> Result<Json<AvatarStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let raw = BASE64
        .decode(body.data.as_bytes())
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid encoding"))?;
    let avatar = Avatar::parse(raw).map_err(|e| match e {
        AvatarError::Empty => error(StatusCode::UNPROCESSABLE_ENTITY, "empty image"),
        AvatarError::TooLarge => error(StatusCode::PAYLOAD_TOO_LARGE, "image too large"),
        AvatarError::UnsupportedFormat => {
            error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported image")
        }
    })?;
    let avatars = PgAvatarRepository::new(state.pool.clone());
    set_avatar(&avatars, current.id(), avatar).await;
    Ok(Json(AvatarStateResponse { has_avatar: true }))
}

pub async fn delete_avatar_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Json<AvatarStateResponse> {
    let avatars = PgAvatarRepository::new(state.pool.clone());
    clear_avatar(&avatars, current.id()).await;
    Json(AvatarStateResponse { has_avatar: false })
}

pub async fn get_avatar_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let users = PgUserRepository::new(state.pool.clone());
    let avatars = PgAvatarRepository::new(state.pool.clone());
    let avatar = get_avatar(&users, &avatars, &username)
        .await
        .map_err(|e| match e {
            AvatarLookupError::UserNotFound => error(StatusCode::NOT_FOUND, "user not found"),
            AvatarLookupError::NoAvatar => error(StatusCode::NOT_FOUND, "no avatar"),
        })?;
    Ok((
        [
            (header::CONTENT_TYPE, avatar.format().content_type()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        avatar.bytes().to_vec(),
    )
        .into_response())
}

const ATOM_CONTENT_TYPE: &str = "application/atom+xml; charset=utf-8";

async fn feed_entries(pool: &PgPool, topics: &[domain::Topic]) -> Vec<crate::feed::FeedEntry> {
    let users = PgUserRepository::new(pool.clone());
    let mut entries = Vec::with_capacity(topics.len());
    for topic in topics {
        let author = match users.find_by_id(topic.author_id()).await {
            Some(user) => user.username().as_str().to_owned(),
            None => continue,
        };
        entries.push(crate::feed::entry_from(topic, &author));
    }
    entries
}

fn feed_response(title: &str, self_url: &str, entries: Vec<crate::feed::FeedEntry>) -> Response {
    let updated = entries
        .first()
        .map(|e| e.updated.clone())
        .unwrap_or_else(|| {
            OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
        });
    let body = crate::feed::render(title, self_url, &updated, &entries);
    ([(header::CONTENT_TYPE, ATOM_CONTENT_TYPE)], body).into_response()
}

pub async fn section_feed_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let slug = Slug::parse(&slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section slug"))?;
    let sections = PgSectionRepository::new(state.pool.clone());
    let topics = PgTopicRepository::new(state.pool.clone());
    let list = list_topics(&sections, &topics, &slug)
        .await
        .map_err(|e| match e {
            ListTopicsError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
        })?;
    let entries = feed_entries(&state.pool, &list).await;
    Ok(feed_response(
        slug.as_str(),
        &format!("/api/sections/{}/feed", slug.as_str()),
        entries,
    ))
}

pub async fn tag_feed_handler(
    State(state): State<AppState>,
    Path(tag): Path<String>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let tag =
        Slug::parse(&tag).map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid tag"))?;
    let topics = PgTopicRepository::new(state.pool.clone());
    let list = list_topics_by_tag(&topics, &tag).await;
    let entries = feed_entries(&state.pool, &list).await;
    Ok(feed_response(
        tag.as_str(),
        &format!("/api/tags/{}/feed", tag.as_str()),
        entries,
    ))
}

pub async fn search_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
) -> Result<Json<Vec<SearchHitResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let query = domain::Query::parse(&params.q)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid query"))?;
    let repo = PgSearchRepository::new(state.pool.clone());
    let hits = search(&repo, &query).await;
    let mut responses = Vec::with_capacity(hits.len());
    for hit in &hits {
        match hit {
            SearchHit::Topic(topic) => responses.push(SearchHitResponse::Topic(
                topic_response(&state.pool, topic).await?.0,
            )),
            SearchHit::Comment(comment) => responses.push(SearchHitResponse::Comment(
                comment_response(&state.pool, comment).await?,
            )),
        }
    }
    Ok(Json(responses))
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
