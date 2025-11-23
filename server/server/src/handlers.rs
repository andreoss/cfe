use crate::auth::{ClientAgent, ClientIp, CurrentUser, OptionalUser, SESSION_COOKIE};
use crate::backend::Backend;
use crate::hasher::Argon2Hasher;
use app::{
    AbuseError, AvatarLookupError, BookmarkError, ChangeEmailError, ChangePasswordError,
    CommitTopicError, CreateGroupError, CreatePollError, CreateTopicError, DeleteError, EditError,
    EnforcementError, ListTopicsError, MarkReadError, MoveTopicError, PollResults,
    PostCommentError, ReactionSummary, RegisterError, ReportError, SetPostscoreError, SignInError,
    UpdateBioError, VoteError, acknowledge_warnings, active_ban, add_bookmark, ban_user,
    block_address, cast_vote, change_password, clear_avatar, clear_reaction, close_report,
    commit_topic, confirm_activation, confirm_email_change, count_open_for_topic, count_unread,
    create_group, create_poll, create_session, create_topic, delete_comment, delete_topic,
    deregister, edit_comment, edit_topic, enforce_posting, get_avatar, get_topic, ignore_user,
    ignored_by, is_bookmarked, lift_address_block, lift_ban, list_address_blocks,
    list_bookmarked_topics, list_comments, list_groups, list_notifications, list_open_reports,
    list_sections, list_topics, list_topics_by_tag, list_warnings, mark_read, move_topic,
    poll_results, post_comment, promote_to_moderator, react, recent_activity, record_post,
    register, remove_bookmark, remove_posts_from_address, report_content, reporter_of,
    request_activation, request_email_change, request_password_reset, reset_password, search,
    set_avatar, set_postscore, sign_in, sign_out as end_session, stop_ignoring,
    summarize_reactions, uncommit_topic, update_bio, warn_user,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use domain::{
    Address, Avatar, AvatarError, Bio, Body, CommentId, ContentItem, Email, GroupId, Page,
    Password, PollId, PollOption, PollOptionId, Question, ReactionTarget, Reason, ReportId,
    ReportKind, Session, SessionId, SessionToken, Slug, TagSet, Title, TopicId, UserId, Username,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

#[derive(Clone)]
pub struct AppState {
    pub backend: Arc<dyn Backend>,
    pub mailer: Arc<dyn app::Mailer + Send + Sync>,
    pub limits: app::Limits,
    pub maintenance: app::MaintenanceSettings,
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
    pub score: i32,
}

#[derive(Serialize)]
pub struct SectionResponse {
    pub slug: String,
    pub title: String,
}

#[derive(Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub section_slug: String,
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct CreateTopicRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Serialize)]
pub struct TopicResponse {
    pub id: String,
    pub section_slug: String,
    pub group_slug: Option<String>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub author_username: String,
    pub created_at: String,
    pub deleted: bool,
    pub deleted_reason: Option<String>,
    pub edited: bool,
    pub postscore: i32,
    pub pending: bool,
    pub open_reports: u64,
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
pub struct SetPostscoreRequest {
    pub postscore: i32,
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

#[derive(Deserialize)]
pub struct PageParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Serialize)]
pub struct PageInfo {
    pub number: u32,
    pub size: u32,
    pub total: u64,
    pub total_pages: u64,
    pub has_next: bool,
    pub has_previous: bool,
}

#[derive(Serialize)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub page: PageInfo,
}

fn to_page(params: &PageParams) -> Result<Page, (StatusCode, Json<ErrorResponse>)> {
    Page::parse(
        params.page.unwrap_or(1),
        params.size.unwrap_or(domain::PAGE_DEFAULT_SIZE),
    )
    .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid page"))
}

fn feed_page() -> Page {
    Page::parse(1, domain::PAGE_MAX_SIZE).expect("feed page is valid")
}

fn paged<T, U>(source: &app::Paged<U>, items: Vec<T>) -> PagedResponse<T> {
    PagedResponse {
        items,
        page: PageInfo {
            number: source.page.number(),
            size: source.page.size(),
            total: source.total,
            total_pages: source.total_pages(),
            has_next: source.has_next(),
            has_previous: source.has_previous(),
        },
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentItemResponse {
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
        score: user.score().value(),
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

async fn start_session(state: &AppState, user_id: UserId) -> SessionToken {
    let sessions = state.backend.sessions();
    let token = generate_token();
    let session = Session::new(
        SessionId::new(uuid::Uuid::new_v4()),
        user_id,
        token.clone(),
        OffsetDateTime::now_utc() + Duration::days(30),
    );
    create_session(&*sessions, session).await;
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
    let repo = state.backend.users();
    let hasher = Argon2Hasher;
    let id = UserId::new(uuid::Uuid::new_v4());
    Password::parse(&body.password)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "password too short"))?;
    let user = register(&*repo, &hasher, id, username, email, &body.password)
        .await
        .map_err(|e| match e {
            RegisterError::UsernameTaken => error(StatusCode::CONFLICT, "username taken"),
            RegisterError::EmailTaken => error(StatusCode::CONFLICT, "email taken"),
        })?;
    let tokens = state.backend.mail_tokens();
    request_activation(
        &*tokens,
        &crate::mail::Sha256Digest,
        &*state.mailer,
        domain::MailTokenId::new(uuid::Uuid::new_v4()),
        &user,
        crate::mail::generate_secret(),
        OffsetDateTime::now_utc(),
    )
    .await;
    let token = start_session(&state, user.id()).await;
    Ok((jar.add(session_cookie(&token)), to_response(&user)))
}

pub async fn sign_in_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<SignInRequest>,
) -> Result<(CookieJar, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&body.username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let repo = state.backend.users();
    let hasher = Argon2Hasher;
    let user = sign_in(&*repo, &hasher, &username, &body.password)
        .await
        .map_err(|e| match e {
            SignInError::NotFound => error(StatusCode::UNAUTHORIZED, "invalid credentials"),
            SignInError::WrongPassword => error(StatusCode::UNAUTHORIZED, "invalid credentials"),
        })?;
    let enforcement = state.backend.enforcement();
    if let Some(ban) = active_ban(&*enforcement, user.id(), OffsetDateTime::now_utc()).await {
        return Err(error(
            StatusCode::FORBIDDEN,
            &format!("account suspended: {}", ban.reason().as_str()),
        ));
    }
    let token = start_session(&state, user.id()).await;
    Ok((jar.add(session_cookie(&token)), to_response(&user)))
}

pub async fn me_handler(CurrentUser(user): CurrentUser) -> Json<UserResponse> {
    to_response(&user)
}

pub async fn sign_out_handler(State(state): State<AppState>, jar: CookieJar) -> CookieJar {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        if let Ok(token) = SessionToken::parse(cookie.value()) {
            let sessions = state.backend.sessions();
            if let Some(session) = sessions.find_by_token(&token).await {
                end_session(&*sessions, session.id()).await;
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
    let repo = state.backend.users();
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
    let repo = state.backend.users();
    let updated = update_bio(&*repo, current.id(), bio)
        .await
        .map_err(|e| match e {
            UpdateBioError::NotFound => error(StatusCode::NOT_FOUND, "user not found"),
        })?;
    Ok(to_profile_response(&updated))
}

async fn topic_response(
    state: &AppState,
    topic: &domain::Topic,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let sections = state.backend.sections();
    let users = state.backend.users();
    let reports = state.backend.reports();
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
    let group_slug = match topic.group_id() {
        Some(group_id) => {
            let groups = state.backend.groups();
            groups
                .find_by_id(group_id)
                .await
                .map(|g| g.slug().as_str().to_owned())
        }
        None => None,
    };
    Ok(Json(TopicResponse {
        id: topic.id().as_uuid().to_string(),
        section_slug: section.slug().as_str().to_owned(),
        group_slug,
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
        postscore: topic.postscore().to_db(),
        pending: topic.is_pending(),
        open_reports: count_open_for_topic(&*reports, topic.id()).await,
    }))
}

pub async fn list_sections_handler(State(state): State<AppState>) -> Json<Vec<SectionResponse>> {
    let repo = state.backend.sections();
    let sections = list_sections(&*repo).await;
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
    OptionalUser(current): OptionalUser,
    axum::extract::Query(params): axum::extract::Query<PageParams>,
) -> Result<Json<PagedResponse<TopicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let slug = Slug::parse(&slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section slug"))?;
    let page = to_page(&params)?;
    let sections = state.backend.sections();
    let topics = state.backend.topics();
    let visibility = app::Visibility::of(current.as_ref());
    let list = list_topics(&*sections, &*topics, &slug, page, visibility)
        .await
        .map_err(|e| match e {
            ListTopicsError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
        })?;
    let mut responses = Vec::with_capacity(list.items.len());
    for topic in &list.items {
        responses.push(topic_response(&state, topic).await?.0);
    }
    Ok(Json(paged(&list, responses)))
}

pub async fn create_topic_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    CurrentUser(current): CurrentUser,
    ClientIp(ip): ClientIp,
    ClientAgent(client): ClientAgent,
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
    let now = OffsetDateTime::now_utc();
    let abuse = state.backend.abuse();
    if let Some(addr) = &ip {
        enforce_posting(&*abuse, &current, addr, now, state.limits)
            .await
            .map_err(abuse_error)?;
    }
    let sections = state.backend.sections();
    let topics = state.backend.topics();
    let groups = state.backend.groups();
    let group_id: Option<GroupId> = match &body.group {
        Some(group_slug) => {
            let group_slug = Slug::parse(group_slug)
                .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid group slug"))?;
            let section = sections
                .find_by_slug(&slug)
                .await
                .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section"))?;
            let group = groups
                .find_by_slug(section.id(), &group_slug)
                .await
                .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "group not found"))?;
            Some(group.id())
        }
        None => None,
    };
    let author = current.id();
    let topic = create_topic(
        &*sections,
        &*topics,
        domain::TopicId::new(uuid::Uuid::new_v4()),
        &slug,
        &current,
        title,
        topic_body,
        tags,
        group_id,
        now,
    )
    .await
    .map_err(|e| match e {
        CreateTopicError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
        CreateTopicError::Restricted => error(StatusCode::FORBIDDEN, "not allowed to post here"),
    })?;
    if let Some(addr) = &ip {
        record_post(
            &*abuse,
            author,
            addr,
            client.as_ref(),
            Some(domain::PostRef::Topic(topic.id())),
            now,
        )
        .await;
    }
    topic_response(&state, &topic).await
}

pub async fn get_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    OptionalUser(current): OptionalUser,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let topic_id = domain::TopicId::new(id);
    let topics = state.backend.topics();
    let topic = get_topic(&*topics, topic_id, app::Visibility::of(current.as_ref()))
        .await
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "topic not found"))?;
    topic_response(&state, &topic).await
}

pub async fn list_topics_by_tag_handler(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    OptionalUser(current): OptionalUser,
    axum::extract::Query(params): axum::extract::Query<PageParams>,
) -> Result<Json<PagedResponse<TopicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let tag =
        Slug::parse(&tag).map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid tag"))?;
    let page = to_page(&params)?;
    let topics = state.backend.topics();
    let visibility = app::Visibility::of(current.as_ref());
    let list = list_topics_by_tag(&*topics, &tag, page, visibility).await;
    let mut responses = Vec::with_capacity(list.items.len());
    for topic in &list.items {
        responses.push(topic_response(&state, topic).await?.0);
    }
    Ok(Json(paged(&list, responses)))
}

#[derive(Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct MoveTopicRequest {
    pub group: String,
}

pub async fn create_group_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<CreateGroupRequest>,
) -> Result<Json<GroupResponse>, (StatusCode, Json<ErrorResponse>)> {
    let slug = Slug::parse(&slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section slug"))?;
    let name = Title::parse(&body.name)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid name"))?;
    let group_slug = Slug::parse(&body.slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid slug"))?;
    let sections = state.backend.sections();
    let groups = state.backend.groups();
    let group = create_group(
        &*sections,
        &*groups,
        domain::GroupId::new(uuid::Uuid::new_v4()),
        &slug,
        name,
        group_slug,
        &current,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        CreateGroupError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
        CreateGroupError::NotAuthorized => error(StatusCode::FORBIDDEN, "not a moderator"),
        CreateGroupError::SlugTaken => error(StatusCode::CONFLICT, "slug already used"),
    })?;
    Ok(Json(GroupResponse {
        id: group.id().as_uuid().to_string(),
        section_slug: slug.as_str().to_owned(),
        name: group.name().as_str().to_owned(),
        slug: group.slug().as_str().to_owned(),
    }))
}

pub async fn list_groups_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<GroupResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let slug = Slug::parse(&slug)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid section slug"))?;
    let sections = state.backend.sections();
    let groups = state.backend.groups();
    let section = sections
        .find_by_slug(&slug)
        .await
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "section not found"))?;
    let list = list_groups(&*groups, section.id()).await;
    Ok(Json(
        list.into_iter()
            .map(|g| GroupResponse {
                id: g.id().as_uuid().to_string(),
                section_slug: slug.as_str().to_owned(),
                name: g.name().as_str().to_owned(),
                slug: g.slug().as_str().to_owned(),
            })
            .collect(),
    ))
}

pub async fn commit_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let topics = state.backend.topics();
    let topic = commit_topic(&*topics, &current, domain::TopicId::new(id))
        .await
        .map_err(|e| match e {
            CommitTopicError::NotFound => error(StatusCode::NOT_FOUND, "topic not found"),
            CommitTopicError::NotAuthorized => error(StatusCode::FORBIDDEN, "not a moderator"),
        })?;
    topic_response(&state, &topic).await
}

pub async fn uncommit_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let topics = state.backend.topics();
    let topic = uncommit_topic(&*topics, &current, domain::TopicId::new(id))
        .await
        .map_err(|e| match e {
            CommitTopicError::NotFound => error(StatusCode::NOT_FOUND, "topic not found"),
            CommitTopicError::NotAuthorized => error(StatusCode::FORBIDDEN, "not a moderator"),
        })?;
    topic_response(&state, &topic).await
}

pub async fn move_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<MoveTopicRequest>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let group_slug = Slug::parse(&body.group)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid group slug"))?;
    let topics = state.backend.topics();
    let groups = state.backend.groups();
    let topic = topics
        .find_by_id(domain::TopicId::new(id))
        .await
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "topic not found"))?;
    let group = groups
        .find_by_slug(topic.section_id(), &group_slug)
        .await
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "group not found"))?;
    let topic = move_topic(
        &*topics,
        &*groups,
        &current,
        domain::TopicId::new(id),
        group.id(),
    )
    .await
    .map_err(|e| match e {
        MoveTopicError::NotFound => error(StatusCode::NOT_FOUND, "topic not found"),
        MoveTopicError::NotAuthorized => error(StatusCode::FORBIDDEN, "not a moderator"),
        MoveTopicError::GroupNotFound => error(StatusCode::UNPROCESSABLE_ENTITY, "group not found"),
        MoveTopicError::WrongSection => {
            error(StatusCode::UNPROCESSABLE_ENTITY, "group in another section")
        }
    })?;
    topic_response(&state, &topic).await
}

async fn comment_response(
    state: &AppState,
    comment: &domain::Comment,
) -> Result<CommentResponse, (StatusCode, Json<ErrorResponse>)> {
    let users = state.backend.users();
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
    axum::extract::Query(params): axum::extract::Query<PageParams>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<PagedResponse<CommentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let page = to_page(&params)?;
    let comments = state.backend.comments();
    let list = list_comments(&*comments, TopicId::new(topic_id), page).await;
    let ignored = match &viewer {
        Some(user) => {
            let enforcement = state.backend.enforcement();
            ignored_by(&*enforcement, user.id()).await
        }
        None => Vec::new(),
    };
    let mut responses = Vec::with_capacity(list.items.len());
    for comment in &list.items {
        let mut response = comment_response(&state, comment).await?;
        if ignored.contains(&comment.author_id()) {
            response.body = String::new();
            response.ignored = true;
        }
        responses.push(response);
    }
    Ok(Json(paged(&list, responses)))
}

pub async fn post_comment_handler(
    State(state): State<AppState>,
    Path(topic_id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    ClientIp(ip): ClientIp,
    ClientAgent(client): ClientAgent,
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
    let now = OffsetDateTime::now_utc();
    let abuse = state.backend.abuse();
    if let Some(addr) = &ip {
        enforce_posting(&*abuse, &current, addr, now, state.limits)
            .await
            .map_err(abuse_error)?;
    }
    let topics = state.backend.topics();
    let comments = state.backend.comments();
    let notifications = state.backend.notifications();
    let sections = state.backend.sections();
    let author = current.id();
    let comment = post_comment(
        &*sections,
        &*topics,
        &*comments,
        &*notifications,
        CommentId::new(uuid::Uuid::new_v4()),
        domain::NotificationId::new(uuid::Uuid::new_v4()),
        TopicId::new(topic_id),
        &current,
        parent_id,
        comment_body,
        now,
    )
    .await
    .map_err(|e| match e {
        PostCommentError::TopicNotFound => error(StatusCode::NOT_FOUND, "topic not found"),
        PostCommentError::ParentNotFound => error(StatusCode::NOT_FOUND, "parent not found"),
        PostCommentError::ParentInDifferentTopic => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "parent in different topic",
        ),
        PostCommentError::Restricted => error(StatusCode::FORBIDDEN, "not allowed to comment"),
    })?;
    if let Some(addr) = &ip {
        record_post(
            &*abuse,
            author,
            addr,
            client.as_ref(),
            Some(domain::PostRef::Comment(comment.id())),
            now,
        )
        .await;
    }
    Ok(Json(comment_response(&state, &comment).await?))
}

pub async fn delete_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<DeleteRequest>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let topics = state.backend.topics();
    let users = state.backend.users();
    let topic = delete_topic(
        &*topics,
        &*users,
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
    topic_response(&state, &topic).await
}

pub async fn delete_comment_handler(
    State(state): State<AppState>,
    Path((_topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<DeleteRequest>,
) -> Result<Json<CommentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let comments = state.backend.comments();
    let users = state.backend.users();
    let comment = delete_comment(
        &*comments,
        &*users,
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
    Ok(Json(comment_response(&state, &comment).await?))
}

pub async fn set_postscore_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<SetPostscoreRequest>,
) -> Result<Json<TopicResponse>, (StatusCode, Json<ErrorResponse>)> {
    let topics = state.backend.topics();
    let topic = set_postscore(
        &*topics,
        &current,
        TopicId::new(id),
        domain::PostScore::from_db(body.postscore),
    )
    .await
    .map_err(|e| match e {
        SetPostscoreError::NotFound => error(StatusCode::NOT_FOUND, "topic not found"),
        SetPostscoreError::NotAuthorized => error(StatusCode::FORBIDDEN, "moderator role required"),
    })?;
    topic_response(&state, &topic).await
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
    let topics = state.backend.topics();
    let topic = edit_topic(
        &*topics,
        &current,
        TopicId::new(id),
        title,
        topic_body,
        tags,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(edit_error)?;
    topic_response(&state, &topic).await
}

pub async fn edit_comment_handler(
    State(state): State<AppState>,
    Path((_topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<EditCommentRequest>,
) -> Result<Json<CommentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let comment_body = Body::parse(&body.body)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid body"))?;
    let comments = state.backend.comments();
    let comment = edit_comment(
        &*comments,
        &current,
        CommentId::new(id),
        comment_body,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(edit_error)?;
    Ok(Json(comment_response(&state, &comment).await?))
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
    state: &AppState,
    notification: &domain::Notification,
) -> Result<NotificationResponse, (StatusCode, Json<ErrorResponse>)> {
    let users = state.backend.users();
    let topics = state.backend.topics();
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
    axum::extract::Query(params): axum::extract::Query<PageParams>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<PagedResponse<NotificationResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let page = to_page(&params)?;
    let repo = state.backend.notifications();
    let list = list_notifications(&*repo, current.id(), page).await;
    let mut responses = Vec::with_capacity(list.items.len());
    for notification in &list.items {
        responses.push(notification_response(&state, notification).await?);
    }
    Ok(Json(paged(&list, responses)))
}

pub async fn unread_count_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Json<UnreadCountResponse> {
    let repo = state.backend.notifications();
    Json(UnreadCountResponse {
        unread: count_unread(&*repo, current.id()).await,
    })
}

pub async fn mark_notification_read_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = state.backend.notifications();
    let notification = mark_read(
        &*repo,
        current.id(),
        domain::NotificationId::new(id),
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        MarkReadError::NotFound => error(StatusCode::NOT_FOUND, "notification not found"),
    })?;
    Ok(Json(notification_response(&state, &notification).await?))
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
    let topics = state.backend.topics();
    let bookmarks = state.backend.bookmarks();
    add_bookmark(
        &*topics,
        &*bookmarks,
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
    let bookmarks = state.backend.bookmarks();
    remove_bookmark(&*bookmarks, current.id(), TopicId::new(id)).await;
    Json(BookmarkStateResponse { bookmarked: false })
}

pub async fn bookmark_state_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Json<BookmarkStateResponse> {
    let bookmarks = state.backend.bookmarks();
    Json(BookmarkStateResponse {
        bookmarked: is_bookmarked(&*bookmarks, current.id(), TopicId::new(id)).await,
    })
}

pub async fn list_bookmarks_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<PageParams>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<PagedResponse<TopicResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let page = to_page(&params)?;
    let bookmarks = state.backend.bookmarks();
    let list = list_bookmarked_topics(&*bookmarks, current.id(), page).await;
    let mut responses = Vec::with_capacity(list.items.len());
    for topic in &list.items {
        responses.push(topic_response(&state, topic).await?.0);
    }
    Ok(Json(paged(&list, responses)))
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
    state: &AppState,
    topic_id: uuid::Uuid,
    comment_id: Option<uuid::Uuid>,
) -> Result<(ReactionTarget, domain::UserId), (StatusCode, Json<ErrorResponse>)> {
    match comment_id {
        Some(id) => {
            let comments = state.backend.comments();
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
            Ok((ReactionTarget::Comment(comment.id()), comment.author_id()))
        }
        None => {
            let topics = state.backend.topics();
            let topic = topics
                .find_by_id(TopicId::new(topic_id))
                .await
                .ok_or_else(|| error(StatusCode::NOT_FOUND, "topic not found"))?;
            Ok((ReactionTarget::Topic(topic.id()), topic.author_id()))
        }
    }
}

async fn reactions_for(
    state: &AppState,
    viewer: Option<&domain::User>,
    target: ReactionTarget,
) -> Json<ReactionsResponse> {
    let repo = state.backend.reactions();
    let summary = summarize_reactions(&*repo, viewer.map(|u| u.id()), target).await;
    Json(to_reactions_response(summary))
}

pub async fn topic_reactions_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (target, _) = reaction_target(&state, id, None).await?;
    Ok(reactions_for(&state, viewer.as_ref(), target).await)
}

pub async fn react_to_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ReactRequest>,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let kind = domain::ReactionKind::parse(&body.kind)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "unknown reaction"))?;
    let (target, author_id) = reaction_target(&state, id, None).await?;
    let repo = state.backend.reactions();
    let users = state.backend.users();
    react(
        &*repo,
        &*users,
        author_id,
        current.id(),
        target,
        kind,
        OffsetDateTime::now_utc(),
    )
    .await;
    Ok(reactions_for(&state, Some(&current), target).await)
}

pub async fn clear_topic_reaction_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (target, author_id) = reaction_target(&state, id, None).await?;
    let repo = state.backend.reactions();
    let users = state.backend.users();
    clear_reaction(&*repo, &*users, author_id, current.id(), target).await;
    Ok(reactions_for(&state, Some(&current), target).await)
}

pub async fn comment_reactions_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (target, _) = reaction_target(&state, topic_id, Some(id)).await?;
    Ok(reactions_for(&state, viewer.as_ref(), target).await)
}

pub async fn react_to_comment_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ReactRequest>,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let kind = domain::ReactionKind::parse(&body.kind)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "unknown reaction"))?;
    let (target, author_id) = reaction_target(&state, topic_id, Some(id)).await?;
    let repo = state.backend.reactions();
    let users = state.backend.users();
    react(
        &*repo,
        &*users,
        author_id,
        current.id(),
        target,
        kind,
        OffsetDateTime::now_utc(),
    )
    .await;
    Ok(reactions_for(&state, Some(&current), target).await)
}

pub async fn clear_comment_reaction_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<ReactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (target, author_id) = reaction_target(&state, topic_id, Some(id)).await?;
    let repo = state.backend.reactions();
    let users = state.backend.users();
    clear_reaction(&*repo, &*users, author_id, current.id(), target).await;
    Ok(reactions_for(&state, Some(&current), target).await)
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
    let polls = state.backend.polls();
    let results = poll_results(&*polls, viewer.map(|u| u.id()), TopicId::new(id))
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
    let topics = state.backend.topics();
    let polls = state.backend.polls();
    create_poll(
        &*topics,
        &*polls,
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
    let results = poll_results(&*polls, Some(current.id()), TopicId::new(id))
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
    let polls = state.backend.polls();
    cast_vote(
        &*polls,
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
    let results = poll_results(&*polls, Some(current.id()), TopicId::new(id))
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

#[derive(Deserialize)]
pub struct BlockAddressRequest {
    pub addr: String,
    pub reason: String,
    pub days: Option<i64>,
}

#[derive(Serialize)]
pub struct AddressBlockResponse {
    pub addr: String,
    pub reason: String,
    pub blocked_at: String,
    pub until: Option<String>,
}

#[derive(Serialize)]
pub struct AddressPostResponse {
    pub username: String,
    pub addr: String,
    pub client: Option<String>,
    pub at: String,
}

#[derive(Deserialize)]
pub struct RemovePostsRequest {
    pub hours: i64,
    pub reason: String,
}

#[derive(Serialize)]
pub struct RemovePostsResponse {
    pub removed: usize,
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
        EnforcementError::NotAuthorized => error(StatusCode::FORBIDDEN, "moderator role required"),
        EnforcementError::UserNotFound => error(StatusCode::NOT_FOUND, "user not found"),
        EnforcementError::NotYourself => error(StatusCode::UNPROCESSABLE_ENTITY, "not yourself"),
    }
}

fn abuse_error(e: AbuseError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        AbuseError::NotAuthorized => error(StatusCode::FORBIDDEN, "moderator role required"),
        AbuseError::AddressBlocked => error(StatusCode::FORBIDDEN, "address is blocked"),
        AbuseError::RateLimited => error(StatusCode::TOO_MANY_REQUESTS, "slow down"),
        AbuseError::SlowMode => error(StatusCode::TOO_MANY_REQUESTS, "slow down"),
    }
}

async fn find_user_id(
    state: &AppState,
    username: &str,
) -> Result<UserId, (StatusCode, Json<ErrorResponse>)> {
    let parsed = Username::parse(username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let users = state.backend.users();
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
    let target = find_user_id(&state, &username).await?;
    let now = OffsetDateTime::now_utc();
    let until = body.days.map(|d| now + Duration::days(d));
    let users = state.backend.users();
    let sessions = state.backend.sessions();
    let enforcement = state.backend.enforcement();
    let ban = ban_user(
        &*users,
        &*sessions,
        &*enforcement,
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
    let target = find_user_id(&state, &username).await?;
    let enforcement = state.backend.enforcement();
    lift_ban(&*enforcement, &current, target)
        .await
        .map_err(enforcement_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_address_blocks_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<Vec<AddressBlockResponse>>, (StatusCode, Json<ErrorResponse>)> {
    if !current.role().is_moderator() {
        return Err(error(StatusCode::FORBIDDEN, "moderator role required"));
    }
    let abuse = state.backend.abuse();
    let blocks = list_address_blocks(&*abuse).await;
    let mut responses = Vec::with_capacity(blocks.len());
    for block in blocks {
        responses.push(AddressBlockResponse {
            addr: block.addr().as_str().to_owned(),
            reason: block.reason().as_str().to_owned(),
            blocked_at: block
                .blocked_at()
                .format(&Rfc3339)
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?,
            until: block.until().and_then(|u| u.format(&Rfc3339).ok()),
        });
    }
    Ok(Json(responses))
}

pub async fn block_address_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<BlockAddressRequest>,
) -> Result<Json<AddressBlockResponse>, (StatusCode, Json<ErrorResponse>)> {
    let addr = Address::parse(&body.addr)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid address"))?;
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let now = OffsetDateTime::now_utc();
    let until = body.days.map(|d| now + Duration::days(d));
    let abuse = state.backend.abuse();
    let block = block_address(&*abuse, &current, addr, reason, now, until)
        .await
        .map_err(abuse_error)?;
    Ok(Json(AddressBlockResponse {
        addr: block.addr().as_str().to_owned(),
        reason: block.reason().as_str().to_owned(),
        blocked_at: block
            .blocked_at()
            .format(&Rfc3339)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?,
        until: block.until().and_then(|u| u.format(&Rfc3339).ok()),
    }))
}

pub async fn lift_address_block_handler(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let addr = Address::parse(&addr)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid address"))?;
    let abuse = state.backend.abuse();
    lift_address_block(&*abuse, &current, &addr)
        .await
        .map_err(abuse_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_address_posts_handler(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    axum::extract::Query(params): axum::extract::Query<PageParams>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<PagedResponse<AddressPostResponse>>, (StatusCode, Json<ErrorResponse>)> {
    if !current.role().is_moderator() {
        return Err(error(StatusCode::FORBIDDEN, "moderator role required"));
    }
    let addr = Address::parse(&addr)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid address"))?;
    let page = to_page(&params)?;
    let abuse = state.backend.abuse();
    let users = state.backend.users();
    let posts = abuse.posts_from_address(&addr, page).await;
    let total = abuse.count_posts_from_address(&addr).await;
    let mut responses = Vec::with_capacity(posts.len());
    for post in &posts {
        let username = match users.find_by_id(post.user_id()).await {
            Some(user) => user.username().as_str().to_owned(),
            None => "unknown".to_owned(),
        };
        responses.push(AddressPostResponse {
            username,
            addr: post.addr().as_str().to_owned(),
            client: post.client().map(|c| c.as_str().to_owned()),
            at: post
                .at()
                .format(&Rfc3339)
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?,
        });
    }
    let list = app::Paged::new(posts, page, total);
    Ok(Json(paged(&list, responses)))
}

pub async fn remove_address_posts_handler(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<RemovePostsRequest>,
) -> Result<Json<RemovePostsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !current.role().is_moderator() {
        return Err(error(StatusCode::FORBIDDEN, "moderator role required"));
    }
    let addr = Address::parse(&addr)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid address"))?;
    if !(1..=168).contains(&body.hours) {
        return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "invalid hours"));
    }
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let now = OffsetDateTime::now_utc();
    let since = now - Duration::hours(body.hours);
    let abuse = state.backend.abuse();
    let topics = state.backend.topics();
    let comments = state.backend.comments();
    let removed = remove_posts_from_address(
        &*abuse, &*topics, &*comments, &current, &addr, since, reason, now,
    )
    .await
    .map_err(abuse_error)?;
    Ok(Json(RemovePostsResponse { removed }))
}

#[derive(Serialize)]
pub struct MaintenanceResponse {
    pub blocked: usize,
    pub dropped: usize,
}

pub async fn run_maintenance_now(state: &AppState) -> MaintenanceResponse {
    let users = state.backend.users();
    let enforcement = state.backend.enforcement();
    let report = app::run_maintenance(
        &*users,
        &*enforcement,
        state.maintenance,
        OffsetDateTime::now_utc(),
    )
    .await;
    MaintenanceResponse {
        blocked: report.blocked,
        dropped: report.dropped,
    }
}

pub async fn run_maintenance_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<MaintenanceResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !current.role().is_moderator() {
        return Err(error(StatusCode::FORBIDDEN, "moderator role required"));
    }
    Ok(Json(run_maintenance_now(&state).await))
}

pub async fn promote_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state, &username).await?;
    let users = state.backend.users();
    let promoted = promote_to_moderator(&*users, &current, target)
        .await
        .map_err(enforcement_error)?;
    Ok(to_response(&promoted))
}

pub async fn warn_user_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<WarnRequest>,
) -> Result<Json<WarningResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let target = find_user_id(&state, &username).await?;
    let users = state.backend.users();
    let enforcement = state.backend.enforcement();
    let warning = warn_user(
        &*users,
        &*enforcement,
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
    let enforcement = state.backend.enforcement();
    Json(
        list_warnings(&*enforcement, current.id())
            .await
            .into_iter()
            .map(|w| WarningResponse {
                id: w.id().as_uuid().to_string(),
                reason: w.reason().as_str().to_owned(),
                created_at: w
                    .created_at()
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| String::new()),
                acknowledged: w.is_acknowledged(),
            })
            .collect(),
    )
}

pub async fn acknowledge_warnings_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> StatusCode {
    let enforcement = state.backend.enforcement();
    acknowledge_warnings(&*enforcement, current.id()).await;
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
    let target = find_user_id(&state, &username).await?;
    let users = state.backend.users();
    let enforcement = state.backend.enforcement();
    ignore_user(&*users, &*enforcement, &current, target)
        .await
        .map_err(enforcement_error)?;
    Ok(Json(IgnoreStateResponse { ignored: true }))
}

pub async fn stop_ignoring_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<IgnoreStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state, &username).await?;
    let enforcement = state.backend.enforcement();
    stop_ignoring(&*enforcement, &current, target).await;
    Ok(Json(IgnoreStateResponse { ignored: false }))
}

pub async fn ignore_state_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<IgnoreStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let target = find_user_id(&state, &username).await?;
    let enforcement = state.backend.enforcement();
    Ok(Json(IgnoreStateResponse {
        ignored: ignored_by(&*enforcement, current.id())
            .await
            .contains(&target),
    }))
}

#[derive(Deserialize)]
pub struct ResetRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetConfirmRequest {
    pub code: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct ChangeEmailRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ConfirmRequest {
    pub code: String,
}

pub async fn request_reset_handler(
    State(state): State<AppState>,
    Json(body): Json<ResetRequest>,
) -> StatusCode {
    if let Ok(address) = Email::parse(&body.email) {
        let users = state.backend.users();
        let tokens = state.backend.mail_tokens();
        request_password_reset(
            &*users,
            &*tokens,
            &crate::mail::Sha256Digest,
            &*state.mailer,
            domain::MailTokenId::new(uuid::Uuid::new_v4()),
            &address,
            crate::mail::generate_secret(),
            OffsetDateTime::now_utc(),
        )
        .await;
    }
    StatusCode::ACCEPTED
}

pub async fn reset_password_handler(
    State(state): State<AppState>,
    Json(body): Json<ResetConfirmRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let new_password = Password::parse(&body.new_password)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid new password"))?;
    let users = state.backend.users();
    let tokens = state.backend.mail_tokens();
    let sessions = state.backend.sessions();
    reset_password(
        &*users,
        &*tokens,
        &*sessions,
        &crate::mail::Sha256Digest,
        &Argon2Hasher,
        &body.code,
        new_password,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid or expired code"))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn request_email_change_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ChangeEmailRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let address = Email::parse(&body.email)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid email"))?;
    let users = state.backend.users();
    let tokens = state.backend.mail_tokens();
    request_email_change(
        &*users,
        &*tokens,
        &crate::mail::Sha256Digest,
        &*state.mailer,
        domain::MailTokenId::new(uuid::Uuid::new_v4()),
        &current,
        &address,
        crate::mail::generate_secret(),
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|e| match e {
        ChangeEmailError::AddressTaken => error(StatusCode::CONFLICT, "address taken"),
    })?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn confirm_email_handler(
    State(state): State<AppState>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let users = state.backend.users();
    let tokens = state.backend.mail_tokens();
    let updated = confirm_email_change(
        &*users,
        &*tokens,
        &crate::mail::Sha256Digest,
        &body.code,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid or expired code"))?;
    Ok(to_response(&updated))
}

pub async fn confirm_activation_handler(
    State(state): State<AppState>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let users = state.backend.users();
    let tokens = state.backend.mail_tokens();
    let updated = confirm_activation(
        &*users,
        &*tokens,
        &crate::mail::Sha256Digest,
        &body.code,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid or expired code"))?;
    Ok(to_response(&updated))
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
    let users = state.backend.users();
    let sessions = state.backend.sessions();
    let updated = change_password(
        &*users,
        &*sessions,
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
    let token = start_session(&state, updated.id()).await;
    Ok((jar.add(session_cookie(&token)), to_response(&updated)))
}

pub async fn deregister_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    CurrentUser(current): CurrentUser,
) -> (CookieJar, Json<UserResponse>) {
    let users = state.backend.users();
    let sessions = state.backend.sessions();
    let avatars = state.backend.avatars();
    clear_avatar(&*avatars, current.id()).await;
    let gone = deregister(&*users, &*sessions, &current, OffsetDateTime::now_utc()).await;
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
    let avatars = state.backend.avatars();
    set_avatar(&*avatars, current.id(), avatar).await;
    Ok(Json(AvatarStateResponse { has_avatar: true }))
}

pub async fn delete_avatar_handler(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
) -> Json<AvatarStateResponse> {
    let avatars = state.backend.avatars();
    clear_avatar(&*avatars, current.id()).await;
    Json(AvatarStateResponse { has_avatar: false })
}

pub async fn get_avatar_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let username = Username::parse(&username)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid username"))?;
    let users = state.backend.users();
    let avatars = state.backend.avatars();
    let avatar = get_avatar(&*users, &*avatars, &username)
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

async fn feed_entries(state: &AppState, topics: &[domain::Topic]) -> Vec<crate::feed::FeedEntry> {
    let users = state.backend.users();
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
    let sections = state.backend.sections();
    let topics = state.backend.topics();
    let list = list_topics(
        &*sections,
        &*topics,
        &slug,
        feed_page(),
        app::Visibility::anonymous(),
    )
    .await
    .map_err(|e| match e {
        ListTopicsError::SectionNotFound => error(StatusCode::NOT_FOUND, "section not found"),
    })?;
    let entries = feed_entries(&state, &list.items).await;
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
    let topics = state.backend.topics();
    let list = list_topics_by_tag(&*topics, &tag, feed_page(), app::Visibility::anonymous()).await;
    let entries = feed_entries(&state, &list.items).await;
    Ok(feed_response(
        tag.as_str(),
        &format!("/api/tags/{}/feed", tag.as_str()),
        entries,
    ))
}

pub async fn activity_handler(
    State(state): State<AppState>,
    OptionalUser(viewer): OptionalUser,
) -> Result<Json<Vec<ContentItemResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let activity = state.backend.activity();
    let enforcement = state.backend.enforcement();
    let items = recent_activity(&*activity, &*enforcement, viewer.map(|u| u.id()), 30).await;
    let mut responses = Vec::with_capacity(items.len());
    for item in &items {
        match item {
            ContentItem::Topic(topic) => responses.push(ContentItemResponse::Topic(
                topic_response(&state, topic).await?.0,
            )),
            ContentItem::Comment(comment) => responses.push(ContentItemResponse::Comment(
                comment_response(&state, comment).await?,
            )),
        }
    }
    Ok(Json(responses))
}

pub async fn search_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
) -> Result<Json<Vec<ContentItemResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let query = domain::Query::parse(&params.q)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid query"))?;
    let repo = state.backend.search();
    let hits = search(&*repo, &query).await;
    let mut responses = Vec::with_capacity(hits.len());
    for hit in &hits {
        match hit {
            ContentItem::Topic(topic) => responses.push(ContentItemResponse::Topic(
                topic_response(&state, topic).await?.0,
            )),
            ContentItem::Comment(comment) => responses.push(ContentItemResponse::Comment(
                comment_response(&state, comment).await?,
            )),
        }
    }
    Ok(Json(responses))
}

#[derive(Deserialize)]
pub struct ReportRequest {
    pub kind: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct ReportResponse {
    pub id: String,
    pub topic_id: String,
    pub comment_id: Option<String>,
    pub reporter_username: String,
    pub kind: String,
    pub reason: String,
    pub created_at: String,
}

fn report_error(e: ReportError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        ReportError::TargetNotFound => error(StatusCode::NOT_FOUND, "target not found"),
        ReportError::AlreadyReported => error(StatusCode::CONFLICT, "already reported"),
        ReportError::TooMany => error(StatusCode::TOO_MANY_REQUESTS, "slow down"),
        ReportError::NotAuthorized => error(StatusCode::FORBIDDEN, "moderator role required"),
        ReportError::NotFound => error(StatusCode::NOT_FOUND, "report not found"),
        ReportError::AlreadyClosed => error(StatusCode::CONFLICT, "report already closed"),
    }
}

async fn report_response(
    state: &AppState,
    report: &domain::Report,
) -> Result<ReportResponse, (StatusCode, Json<ErrorResponse>)> {
    let users = state.backend.users();
    let reporter = reporter_of(&*users, report.reporter_id())
        .await
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "reporter missing"))?;
    let created_at = report
        .created_at()
        .format(&Rfc3339)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "bad timestamp"))?;
    Ok(ReportResponse {
        id: report.id().as_uuid().to_string(),
        topic_id: report.target().topic_id().as_uuid().to_string(),
        comment_id: report
            .target()
            .comment_id()
            .map(|c| c.as_uuid().to_string()),
        reporter_username: reporter.username().as_str().to_owned(),
        kind: report.kind().as_str().to_owned(),
        reason: report.reason().as_str().to_owned(),
        created_at,
    })
}

async fn create_report(
    state: &AppState,
    reporter: &domain::User,
    topic_id: TopicId,
    comment_id: Option<CommentId>,
    body: ReportRequest,
) -> Result<Json<ReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let kind = ReportKind::parse(&body.kind)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid kind"))?;
    let reason = Reason::parse(&body.reason)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid reason"))?;
    let reports = state.backend.reports();
    let topics = state.backend.topics();
    let comments = state.backend.comments();
    let report = report_content(
        &*reports,
        &*topics,
        &*comments,
        ReportId::new(uuid::Uuid::new_v4()),
        reporter,
        topic_id,
        comment_id,
        kind,
        reason,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(report_error)?;
    Ok(Json(report_response(state, &report).await?))
}

pub async fn report_topic_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ReportRequest>,
) -> Result<Json<ReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    create_report(&state, &current, TopicId::new(id), None, body).await
}

pub async fn report_comment_handler(
    State(state): State<AppState>,
    Path((topic_id, id)): Path<(uuid::Uuid, uuid::Uuid)>,
    CurrentUser(current): CurrentUser,
    Json(body): Json<ReportRequest>,
) -> Result<Json<ReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    create_report(
        &state,
        &current,
        TopicId::new(topic_id),
        Some(CommentId::new(id)),
        body,
    )
    .await
}

pub async fn list_reports_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<PageParams>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<PagedResponse<ReportResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let page = to_page(&params)?;
    let reports = state.backend.reports();
    let list = list_open_reports(&*reports, &current, page)
        .await
        .map_err(report_error)?;
    let mut responses = Vec::with_capacity(list.items.len());
    for report in &list.items {
        responses.push(report_response(&state, report).await?);
    }
    Ok(Json(paged(&list, responses)))
}

pub async fn close_report_handler(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    CurrentUser(current): CurrentUser,
) -> Result<Json<ReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let reports = state.backend.reports();
    let report = close_report(
        &*reports,
        &current,
        ReportId::new(id),
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(report_error)?;
    Ok(Json(report_response(&state, &report).await?))
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
