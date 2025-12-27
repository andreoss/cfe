use axum::{
    Form, Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::Engine as _;
use base64::prelude::BASE64_STANDARD as BASE64;
use serde::Deserialize;
use std::collections::BTreeMap;

use client::Comment;

use crate::{
    App,
    html::{self, Chrome},
    theme::Theme,
    token::{Guard, cookie_of},
};

const STYLESHEET: &str = include_str!("../static/style.css");
const LAST_PAGE: u32 = 10_000;
const SESSION_COOKIE: &str = "session";

pub fn router(app: App) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/", get(index))
        .route("/sections/{slug}", get(section))
        .route("/sections/{slug}/feed", get(section_feed))
        .route("/topics/{id}", get(topic))
        .route("/topics/{id}/reply", get(reply_form))
        .route("/topics/{id}/comments", post(add_comment))
        .route(
            "/topics/{id}/edit",
            get(subject_edit_form).post(edit_subject),
        )
        .route("/topics/{id}/remove", get(subject_removal_form))
        .route("/topics/{id}/delete", post(remove_subject))
        .route(
            "/topics/{id}/restore",
            get(subject_restore_form).post(restore_subject),
        )
        .route("/topics/{id}/publish", post(publish_subject))
        .route("/topics/{id}/sticky", post(pin_subject))
        .route("/topics/{id}/front", post(front_subject))
        .route("/topics/{id}/commit", post(commit_subject))
        .route("/topics/{id}/score", post(score_subject))
        .route("/topics/{id}/group", post(move_subject))
        .route("/topics/{id}/resolved", post(resolve_subject))
        .route(
            "/topics/{id}/images",
            get(subject_pictures).post(attach_picture),
        )
        .route("/topics/{id}/images/{picture}", get(picture))
        .route("/topics/{id}/images/{picture}/remove", post(remove_picture))
        .route("/topics/{id}/poll/new", get(poll_form))
        .route("/topics/{id}/poll", post(add_poll))
        .route("/topics/{id}/poll/vote", post(cast_vote))
        .route("/topics/{id}/react", post(react_to_subject))
        .route("/topics/{id}/reactions/clear", post(clear_subject_reaction))
        .route(
            "/topics/{id}/comments/{remark}/react",
            post(react_to_remark),
        )
        .route(
            "/topics/{id}/comments/{remark}/reactions/clear",
            post(clear_remark_reaction),
        )
        .route("/topics/{id}/history", get(history))
        .route("/topics/{id}/history/{version}", get(difference))
        .route(
            "/topics/{id}/comments/{remark}/edit",
            get(remark_edit_form).post(edit_remark),
        )
        .route(
            "/topics/{id}/comments/{remark}/remove",
            get(remark_removal_form),
        )
        .route("/topics/{id}/comments/{remark}/delete", post(remove_remark))
        .route(
            "/topics/{id}/comments/{remark}/restore",
            get(remark_restore_form).post(restore_remark),
        )
        .route("/sections/{slug}/post", get(subject_form).post(add_subject))
        .route("/search", get(search))
        .route("/archive", get(archive))
        .route("/archive/{year}/{month}", get(archive_month))
        .route("/activity", get(activity))
        .route("/bookmarks", get(bookmarks))
        .route("/watched", get(watched))
        .route("/notifications", get(notifications))
        .route("/notifications/{id}/read", post(read_notification))
        .route("/tags/{tag}", get(tag))
        .route("/tags/{tag}/feed", get(tag_feed))
        .route("/tags/{tag}/follow", post(follow_tag))
        .route("/tags/{tag}/unfollow", post(unfollow_tag))
        .route("/tags/{tag}/describe", post(describe_tag))
        .route("/register", get(register_form).post(register))
        .route("/sign-in", get(sign_in_form).post(sign_in))
        .route("/sign-out", post(sign_out))
        .route("/u/{username}", get(profile))
        .route("/u/{username}/bio", post(change_bio))
        .route("/u/{username}/avatar", get(avatar))
        .route(
            "/settings/password",
            get(password_form).post(change_password),
        )
        .route("/settings/email", get(email_form).post(request_email))
        .route("/settings/email/confirm", post(confirm_email))
        .route(
            "/settings/deregister",
            get(deregister_form).post(deregister),
        )
        .route("/forgot", get(forgot_form).post(request_reset))
        .route("/forgot/confirm", get(reset_form).post(confirm_reset))
        .route("/activate", get(activate_form).post(activate))
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

async fn index(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/").await;
    match app.sections().await {
        Ok(sections) => render(
            &guard,
            StatusCode::OK,
            html::page(&chrome, "Sections", &html::section_list(&sections)),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
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
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = section_address(&slug, number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(board) = (match app.section(&slug).await {
        Ok(board) => board,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    }) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such section",
                "This board has no section at that address.",
            ),
        );
    };
    match app.topics(&slug, number).await {
        Ok(topics) => render(
            &guard,
            StatusCode::OK,
            html::page(&chrome, &board.title, &html::topic_list(&board, &topics)),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn topic(
    State(app): State<App>,
    Path(id): Path<String>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = topic_address(&id, number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let session = session_of(&headers);
    let Some(subject) = (match app.subject(session.as_deref(), &id).await {
        Ok(subject) => subject,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    }) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such subject",
                "This board has no subject at that address.",
            ),
        );
    };
    let holding = session.is_some();
    let pictures = app.images(&id).await.unwrap_or_default();
    let groups = app.groups(&subject.section_slug).await.unwrap_or_default();
    let poll = app.poll(session.as_deref(), &id).await.unwrap_or_default();
    let reactions = app
        .topic_reactions(session.as_deref(), &id)
        .await
        .ok()
        .filter(|found| !found.counts.is_empty());
    match app.comments(session.as_deref(), &id, number).await {
        Ok(comments) => {
            let mut remark_reactions = BTreeMap::new();
            for remark in &comments.items {
                let found = app
                    .comment_reactions(session.as_deref(), &id, &remark.id)
                    .await;
                if let Some(found) = found.ok().filter(|found| !found.counts.is_empty()) {
                    remark_reactions.insert(remark.id.clone(), found);
                }
            }
            render(
                &guard,
                StatusCode::OK,
                html::page(
                    &chrome,
                    &subject.title,
                    &html::subject_page(
                        &subject,
                        &comments,
                        &html::SubjectView {
                            holding,
                            standing: chrome.standing().map(str::to_owned),
                            writer: chrome.account_name() == Some(subject.author_username.as_str()),
                            token: guard.token().to_owned(),
                            groups,
                            pictures,
                            poll,
                            reactions,
                            remark_reactions,
                        },
                    ),
                ),
            )
        }
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

#[derive(Deserialize)]
pub struct FlagForm {
    token: Option<String>,
    on: Option<String>,
}

#[derive(Deserialize)]
pub struct ScoreForm {
    token: Option<String>,
    score: Option<String>,
}

#[derive(Deserialize)]
pub struct MoveForm {
    token: Option<String>,
    group: Option<String>,
}

fn flag_of(raw: &Option<String>) -> bool {
    matches!(raw.as_deref().map(str::trim), Some("yes"))
}

struct Holding {
    guard: Guard,
    chrome: Chrome,
    address: String,
    session: String,
}

async fn holding(
    app: &App,
    id: &str,
    headers: &HeaderMap,
    token: Option<&str>,
) -> Result<Holding, Response> {
    let guard = Guard::new(headers);
    let theme = theme_of(headers, app);
    let address = topic_address(id, 1);
    let chrome = chrome_of(&guard, app, headers, theme, &address).await;
    if !guard.allows(token) {
        return Err(render(&guard, StatusCode::FORBIDDEN, expired(&chrome)));
    }
    let Some(session) = session_of(headers) else {
        return Err(sign_in_first(&guard, &address));
    };
    Ok(Holding {
        guard,
        chrome,
        address,
        session,
    })
}

fn settled(holding: &Holding, title: &str, outcome: Result<(), client::ClientError>) -> Response {
    match outcome {
        Ok(()) => went(&holding.guard, &None, &holding.address),
        Err(error) => render(
            &holding.guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(&holding.chrome, title, &error.to_string()),
        ),
    }
}

async fn publish_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = app.publish(&held.session, &id).await.map(|_| ());
    settled(&held, "The subject was not published", outcome)
}

async fn pin_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<FlagForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = app.sticky(&held.session, &id, flag_of(&form.on)).await;
    settled(&held, "The subject was not pinned", outcome.map(|_| ()))
}

async fn front_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<FlagForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = app.off_front(&held.session, &id, !flag_of(&form.on)).await;
    settled(&held, "The front page was not changed", outcome.map(|_| ()))
}

async fn commit_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<FlagForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = match flag_of(&form.on) {
        true => app.commit(&held.session, &id).await,
        false => app.uncommit(&held.session, &id).await,
    };
    settled(&held, "The subject was not committed", outcome.map(|_| ()))
}

async fn resolve_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<FlagForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = app.resolved(&held.session, &id, flag_of(&form.on)).await;
    settled(&held, "The subject was not marked", outcome.map(|_| ()))
}

async fn score_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<ScoreForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let score = form
        .score
        .as_deref()
        .map(str::trim)
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    let outcome = app.postscore(&held.session, &id, score).await;
    settled(&held, "The score was not set", outcome.map(|_| ()))
}

async fn move_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<MoveForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let group = form.group.clone().unwrap_or_default();
    let outcome = app.move_to(&held.session, &id, &group).await;
    settled(&held, "The subject was not moved", outcome.map(|_| ()))
}

async fn subject_pictures(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = format!("/topics/{}/images", client::encode_path(&id));
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let pictures = match app.images(&id).await {
        Ok(pictures) => pictures,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            &format!("Pictures of {}", subject.title),
            &html::picture_page(&subject, &pictures, &chrome),
        ),
    )
}

async fn picture(
    State(app): State<App>,
    Path((id, picture_id)): Path<(String, String)>,
) -> Response {
    match app.image(&id, &picture_id).await {
        Ok(Some(picture)) => (
            [
                (header::CONTENT_TYPE, picture.content_type().to_owned()),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
            ],
            picture.bytes().to_vec(),
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn attach_picture(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = format!("/topics/{}/images", client::encode_path(&id));
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let (token, bytes) = match uploaded(multipart).await {
        Some(found) => found,
        None => {
            return render(
                &guard,
                StatusCode::BAD_REQUEST,
                html::message(
                    &chrome,
                    "No picture was sent",
                    "Choose a picture and send again.",
                ),
            );
        }
    };
    if !guard.allows(token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let encoded = BASE64.encode(bytes.as_slice());
    match app.attach_image(&session, &id, &encoded).await {
        Ok(_) => went(&guard, &None, &address),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(&chrome, "The picture was not put up", &error.to_string()),
        ),
    }
}

async fn uploaded(mut multipart: Multipart) -> Option<(Option<String>, Vec<u8>)> {
    let mut token = None;
    let mut bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "token" => token = field.text().await.ok(),
            _ => {
                if bytes.is_none() {
                    bytes = field.bytes().await.ok().map(|found| found.to_vec());
                }
            }
        }
    }
    bytes.map(|bytes| (token, bytes))
}

async fn remove_picture(
    State(app): State<App>,
    Path((id, picture_id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = format!("/topics/{}/images", client::encode_path(&id));
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    match app.remove_image(&session, &id, &picture_id).await {
        Ok(()) => went(&guard, &None, &address),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(
                &chrome,
                "The picture was not taken down",
                &error.to_string(),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct PollForm {
    token: Option<String>,
    question: Option<String>,
    options: Option<String>,
}

#[derive(Deserialize)]
pub struct VoteForm {
    token: Option<String>,
    option: Option<String>,
}

#[derive(Deserialize)]
pub struct ReactForm {
    token: Option<String>,
    kind: Option<String>,
}

async fn poll_form(State(app): State<App>, Path(id): Path<String>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::poll_form_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    if app
        .poll(session_of(&headers).as_deref(), &id)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return render(
            &guard,
            StatusCode::CONFLICT,
            html::message(
                &chrome,
                "The subject has a poll",
                "A subject carries one poll, and this one has it already.",
            ),
        );
    }
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            &format!("A poll on {}", subject.title),
            &html::poll_form_page(&subject, &chrome, None),
        ),
    )
}

async fn add_poll(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<PollForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let question = form.question.clone().unwrap_or_default().trim().to_owned();
    let options: Vec<&str> = form
        .options
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if question.is_empty() || options.len() < 2 {
        let subject = match app.subject(Some(&held.session), &id).await {
            Ok(Some(subject)) => subject,
            _ => return went(&held.guard, &None, &held.address),
        };
        return render(
            &held.guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &held.chrome,
                &format!("A poll on {}", subject.title),
                &html::poll_form_page(
                    &subject,
                    &held.chrome,
                    Some("A poll needs a question and at least two ways to answer."),
                ),
            ),
        );
    }
    let outcome = app
        .create_poll(&held.session, &id, &question, &options)
        .await
        .map(|_| ());
    settled(&held, "The poll was not put up", outcome)
}

async fn cast_vote(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<VoteForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let Some(option) = form.option.as_deref().map(str::trim) else {
        return went(&held.guard, &None, &held.address);
    };
    if option.is_empty() {
        return went(&held.guard, &None, &held.address);
    }
    let outcome = app.vote(&held.session, &id, option).await.map(|_| ());
    settled(&held, "The vote was not counted", outcome)
}

async fn react_to_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<ReactForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let Some(kind) = kind_of(form.kind.as_deref()) else {
        return went(&held.guard, &None, &held.address);
    };
    let outcome = app
        .react_to_topic(&held.session, &id, kind)
        .await
        .map(|_| ());
    settled(&held, "The reaction was not kept", outcome)
}

async fn clear_subject_reaction(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = app
        .clear_topic_reaction(&held.session, &id)
        .await
        .map(|_| ());
    settled(&held, "The reaction was not taken away", outcome)
}

async fn react_to_remark(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
    Form(form): Form<ReactForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let Some(kind) = kind_of(form.kind.as_deref()) else {
        return went(&held.guard, &None, &held.address);
    };
    let outcome = app
        .react_to_comment(&held.session, &id, &remark, kind)
        .await
        .map(|_| ());
    settled(&held, "The reaction was not kept", outcome)
}

async fn clear_remark_reaction(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let held = match holding(&app, &id, &headers, form.token.as_deref()).await {
        Ok(held) => held,
        Err(answer) => return answer,
    };
    let outcome = app
        .clear_comment_reaction(&held.session, &id, &remark)
        .await
        .map(|_| ());
    settled(&held, "The reaction was not taken away", outcome)
}

fn kind_of(raw: Option<&str>) -> Option<&str> {
    let kind = raw.map(str::trim)?;
    html::REACTIONS
        .iter()
        .find(|known| **known == kind)
        .copied()
}

async fn subject_form(
    State(app): State<App>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::subject_form_address(&slug);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let Some(section) = (match app.section(&slug).await {
        Ok(section) => section,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    }) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such section",
                "This board has no section at that address.",
            ),
        );
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Write a subject",
            &html::subject_form_page(&section, &chrome, None),
        ),
    )
}

#[derive(Deserialize)]
pub struct SubjectForm {
    token: Option<String>,
    title: String,
    body: String,
    tags: Option<String>,
    draft: Option<String>,
}

async fn add_subject(
    State(app): State<App>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(form): Form<SubjectForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::subject_form_address(&slug);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let Some(section) = (match app.section(&slug).await {
        Ok(section) => section,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    }) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such section",
                "This board has no section at that address.",
            ),
        );
    };
    let tags = tags_of(form.tags.as_deref().unwrap_or(""));
    let mut body = client::NewSubject::new(form.title.trim(), &form.body);
    body = body.tags(&tags.iter().map(String::as_str).collect::<Vec<&str>>());
    if form.draft.is_some() {
        body = body.draft();
    }
    match app.create_topic(&session, &slug, &body).await {
        Ok(subject) => see_other(&guard, &topic_address(&subject.id, 1)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Write a subject",
                &html::subject_form_page(&section, &chrome, Some(&error.to_string())),
            ),
        ),
    }
}

async fn reply_form(
    State(app): State<App>,
    Path(id): Path<String>,
    Query(query): Query<ReplyQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = reply_address(&id, query.parent.as_deref());
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let Some(subject) = (match app.subject(session_of(&headers).as_deref(), &id).await {
        Ok(subject) => subject,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    }) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such subject",
                "This board has no subject at that address.",
            ),
        );
    };
    let answered = answered_of(&app, &id, query.parent.as_deref(), &headers).await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Answer",
            &html::remark_form_page(&subject, answered.as_ref(), &chrome, None),
        ),
    )
}

#[derive(Deserialize)]
pub struct ReplyQuery {
    parent: Option<String>,
}

#[derive(Deserialize)]
pub struct RemarkForm {
    token: Option<String>,
    body: String,
    parent_id: Option<String>,
}

async fn add_comment(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<RemarkForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let parent = form
        .parent_id
        .as_deref()
        .map(str::trim)
        .filter(|parent| !parent.is_empty());
    let address = reply_address(&id, parent);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let mut body = client::NewRemark::new(&form.body);
    if let Some(parent_id) = parent {
        body = body.reply_to(parent_id);
    }
    match app.create_comment(&session, &id, &body).await {
        Ok(comment) => see_other(
            &guard,
            &format!(
                "/topics/{}#remark-{}",
                client::encode_path(&id),
                client::encode_path(&comment.id)
            ),
        ),
        Err(error) => {
            let subject: Option<client::Subject> = app
                .subject(session_of(&headers).as_deref(), &id)
                .await
                .unwrap_or_default();
            let page = match subject {
                Some(subject) => {
                    let answered = answered_of(&app, &id, parent, &headers).await;
                    html::remark_form_page(
                        &subject,
                        answered.as_ref(),
                        &chrome,
                        Some(&error.to_string()),
                    )
                }
                None => html::message(&chrome, "No such subject", &error.to_string()),
            };
            render(
                &guard,
                StatusCode::UNPROCESSABLE_ENTITY,
                html::page(&chrome, "Answer", &page),
            )
        }
    }
}

#[derive(Deserialize)]
pub struct SubjectEditForm {
    token: Option<String>,
    title: String,
    body: String,
    tags: Option<String>,
    minor: Option<String>,
}

async fn subject_edit_form(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::subject_edit_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Change a subject",
            &html::subject_edit_page(&subject, &chrome, None),
        ),
    )
}

async fn edit_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<SubjectEditForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::subject_edit_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let tags = tags_of(form.tags.as_deref().unwrap_or(""));
    let mut body = client::SubjectEdit::new(form.title.trim(), &form.body);
    body = body.tags(&tags.iter().map(String::as_str).collect::<Vec<&str>>());
    if form.minor.is_some() {
        body = body.minor();
    }
    match app.edit_topic(&session, &id, &body).await {
        Ok(_) => see_other(&guard, &topic_address(&id, 1)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Change a subject",
                &html::subject_edit_page(&subject, &chrome, Some(&error.to_string())),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct RemarkEditForm {
    token: Option<String>,
    body: String,
}

async fn remark_edit_form(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::remark_edit_address(&id, &remark);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let said = match held_remark(&app, &id, &remark, &guard, &chrome, &headers).await {
        Ok(said) => said,
        Err(answer) => return answer,
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Change a remark",
            &html::remark_edit_page(&subject, &said, &chrome, None),
        ),
    )
}

async fn edit_remark(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
    Form(form): Form<RemarkEditForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::remark_edit_address(&id, &remark);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let said = match held_remark(&app, &id, &remark, &guard, &chrome, &headers).await {
        Ok(said) => said,
        Err(answer) => return answer,
    };
    match app.edit_comment(&session, &id, &remark, &form.body).await {
        Ok(_) => see_other(&guard, &remark_anchor(&id, &remark)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Change a remark",
                &html::remark_edit_page(&subject, &said, &chrome, Some(&error.to_string())),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct RemovalForm {
    token: Option<String>,
    reason: String,
}

async fn subject_removal_form(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::removal_form_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Remove a subject",
            &html::removal_page(&subject, &chrome, None),
        ),
    )
}

async fn remove_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<RemovalForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::removal_form_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let body = client::Removal::new(form.reason.trim());
    match app.delete_topic(&session, &id, &body).await {
        Ok(_) => see_other(&guard, &topic_address(&id, 1)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Remove a subject",
                &html::removal_page(&subject, &chrome, Some(&error.to_string())),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct RestoreForm {
    token: Option<String>,
}

async fn subject_restore_form(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::restore_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Bring a subject back",
            &html::restore_page(&subject, &chrome, None),
        ),
    )
}

async fn restore_subject(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<RestoreForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::restore_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    match app.restore_topic(&session, &id).await {
        Ok(_) => see_other(&guard, &topic_address(&id, 1)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Bring a subject back",
                &html::restore_page(&subject, &chrome, Some(&error.to_string())),
            ),
        ),
    }
}

async fn remark_removal_form(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::remark_removal_form_address(&id, &remark);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let said = match held_remark(&app, &id, &remark, &guard, &chrome, &headers).await {
        Ok(said) => said,
        Err(answer) => return answer,
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Remove a remark",
            &html::remark_removal_page(&subject, &said, &chrome, None),
        ),
    )
}

async fn remove_remark(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
    Form(form): Form<RemovalForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::remark_removal_form_address(&id, &remark);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let said = match held_remark(&app, &id, &remark, &guard, &chrome, &headers).await {
        Ok(said) => said,
        Err(answer) => return answer,
    };
    let body = client::Removal::new(form.reason.trim());
    match app.delete_comment(&session, &id, &remark, &body).await {
        Ok(_) => see_other(&guard, &remark_anchor(&id, &remark)),
        Err(error) => {
            let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
                Ok(subject) => subject,
                Err(answer) => return answer,
            };
            render(
                &guard,
                StatusCode::UNPROCESSABLE_ENTITY,
                html::page(
                    &chrome,
                    "Remove a remark",
                    &html::remark_removal_page(&subject, &said, &chrome, Some(&error.to_string())),
                ),
            )
        }
    }
}

async fn remark_restore_form(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::remark_restore_address(&id, &remark);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(_session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let said = match held_remark(&app, &id, &remark, &guard, &chrome, &headers).await {
        Ok(said) => said,
        Err(answer) => return answer,
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Bring a remark back",
            &html::remark_restore_page(&subject, &said, &chrome, None),
        ),
    )
}

async fn restore_remark(
    State(app): State<App>,
    Path((id, remark)): Path<(String, String)>,
    headers: HeaderMap,
    Form(form): Form<RestoreForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::remark_restore_address(&id, &remark);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let said = match held_remark(&app, &id, &remark, &guard, &chrome, &headers).await {
        Ok(said) => said,
        Err(answer) => return answer,
    };
    match app.restore_comment(&session, &id, &remark).await {
        Ok(_) => see_other(&guard, &remark_anchor(&id, &remark)),
        Err(error) => {
            let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
                Ok(subject) => subject,
                Err(answer) => return answer,
            };
            render(
                &guard,
                StatusCode::UNPROCESSABLE_ENTITY,
                html::page(
                    &chrome,
                    "Bring a remark back",
                    &html::remark_restore_page(&subject, &said, &chrome, Some(&error.to_string())),
                ),
            )
        }
    }
}

async fn history(State(app): State<App>, Path(id): Path<String>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::history_address(&id);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    match app.topic_history(&id).await {
        Ok(versions) => render(
            &guard,
            StatusCode::OK,
            html::page(
                &chrome,
                "What changed",
                &html::history_page(&subject, &versions),
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn difference(
    State(app): State<App>,
    Path((id, version)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = html::version_address(&id, &version);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let subject = match held_subject(&app, &id, &guard, &chrome, &headers).await {
        Ok(subject) => subject,
        Err(answer) => return answer,
    };
    let versions = match app.topic_history(&id).await {
        Ok(versions) => versions,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    };
    let Some(known) = versions.iter().find(|known| known.id == version) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such version",
                "This subject has no version at that address.",
            ),
        );
    };
    match app.topic_difference(&id, &version).await {
        Ok(changes) => render(
            &guard,
            StatusCode::OK,
            html::page(
                &chrome,
                "What changed",
                &html::difference_page(&subject, known, &changes),
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn held_subject(
    app: &App,
    id: &str,
    guard: &Guard,
    chrome: &Chrome,
    headers: &HeaderMap,
) -> Result<client::Subject, Response> {
    match app.subject(session_of(headers).as_deref(), id).await {
        Ok(Some(subject)) => Ok(subject),
        Ok(None) => Err(render(
            guard,
            StatusCode::NOT_FOUND,
            html::message(
                chrome,
                "No such subject",
                "This board has no subject at that address.",
            ),
        )),
        Err(error) => Err(render(
            guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(chrome, &error),
        )),
    }
}

async fn held_remark(
    app: &App,
    topic: &str,
    id: &str,
    guard: &Guard,
    chrome: &Chrome,
    headers: &HeaderMap,
) -> Result<Comment, Response> {
    let mut number = 1u32;
    loop {
        let session = session_of(headers);
        let remarks = match app.comments(session.as_deref(), topic, number).await {
            Ok(remarks) => remarks,
            Err(error) => {
                return Err(render(
                    guard,
                    StatusCode::SERVICE_UNAVAILABLE,
                    unavailable(chrome, &error),
                ));
            }
        };
        let known = remarks.items.into_iter().find(|said| said.id == id);
        if let Some(said) = known {
            return Ok(said);
        }
        if !remarks.page.has_next || number >= LAST_PAGE {
            return Err(render(
                guard,
                StatusCode::NOT_FOUND,
                html::message(chrome, "No such remark", "This subject has no such remark."),
            ));
        }
        number += 1;
    }
}

fn remark_anchor(topic: &str, remark: &str) -> String {
    format!(
        "/topics/{}#remark-{}",
        client::encode_path(topic),
        client::encode_path(remark)
    )
}

async fn answered_of(
    app: &App,
    id: &str,
    parent: Option<&str>,
    headers: &HeaderMap,
) -> Option<Comment> {
    let parent = parent?;
    let session = session_of(headers);
    let remarks = app.comments(session.as_deref(), id, 1).await.ok()?;
    remarks.items.into_iter().find(|remark| remark.id == parent)
}

fn reply_address(id: &str, parent: Option<&str>) -> String {
    match parent {
        Some(parent) => format!(
            "{}?parent={}",
            html::reply_address(id),
            client::encode_path(parent)
        ),
        None => html::reply_address(id),
    }
}

fn tags_of(raw: &str) -> Vec<String> {
    raw.split(|byte: char| byte == ',' || byte.is_whitespace())
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_owned())
        .collect()
}

fn see_other(guard: &Guard, address: &str) -> Response {
    match guard.set_cookie() {
        Some(cookie) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, cookie),
                (header::LOCATION, address.to_owned()),
            ],
        )
            .into_response(),
        None => (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, address.to_owned())],
        )
            .into_response(),
    }
}

async fn section_feed(
    State(app): State<App>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let feed = match app.section_feed(&slug).await {
        Ok(feed) => feed,
        Err(error) => return xml_unavailable(&error),
    };
    match feed {
        Some(feed) => xml_response(&feed),
        None => {
            let theme = theme_of(&headers, &app);
            let chrome = chrome_of(&guard, &app, &headers, theme, "/").await;
            render(
                &guard,
                StatusCode::NOT_FOUND,
                html::message(
                    &chrome,
                    "No such feed",
                    "This board has no feed at that address.",
                ),
            )
        }
    }
}

async fn tag_feed(
    State(app): State<App>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let feed = match app.tag_feed(&name).await {
        Ok(feed) => feed,
        Err(error) => return xml_unavailable(&error),
    };
    match feed {
        Some(feed) => xml_response(&feed),
        None => {
            let theme = theme_of(&headers, &app);
            let chrome = chrome_of(&guard, &app, &headers, theme, "/").await;
            render(
                &guard,
                StatusCode::NOT_FOUND,
                html::message(
                    &chrome,
                    "No such feed",
                    "This board has no feed at that address.",
                ),
            )
        }
    }
}

fn xml_response(feed: &client::Feed) -> Response {
    (
        [
            (header::CONTENT_TYPE, feed.content_type().to_owned()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        feed.body().to_owned(),
    )
        .into_response()
}

fn xml_unavailable(error: &client::ClientError) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8".to_owned())],
        error.to_string(),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    scope: Option<String>,
    order: Option<String>,
}

async fn search(
    State(app): State<App>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let words = query.q.unwrap_or_default();
    let criteria = client::Criteria::parse(&words, query.scope.as_deref(), query.order.as_deref());
    let address = crate::search_address(&criteria);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let hits = if words.trim().is_empty() {
        None
    } else {
        match app.search(&criteria).await {
            Ok(hits) => Some(hits),
            Err(error) => {
                return render(
                    &guard,
                    StatusCode::SERVICE_UNAVAILABLE,
                    unavailable(&chrome, &error),
                );
            }
        }
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Search",
            &html::search_page(&criteria, hits.as_deref()),
        ),
    )
}

async fn archive(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/archive").await;
    let session = session_of(&headers);
    match app.archive(session.as_deref()).await {
        Ok(months) => render(
            &guard,
            StatusCode::OK,
            html::page(&chrome, "Archive", &html::archive_page(&months)),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn activity(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/activity").await;
    let session = session_of(&headers);
    match app.activity(session.as_deref()).await {
        Ok(hits) => render(
            &guard,
            StatusCode::OK,
            html::page(&chrome, "Activity", &html::activity_page(&hits)),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn bookmarks(
    State(app): State<App>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = kept_address("/bookmarks", number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/bookmarks");
    };
    match app.bookmarks(&session, number).await {
        Ok(topics) => render(
            &guard,
            StatusCode::OK,
            html::page(&chrome, "Bookmarks", &html::bookmarks_page(&topics)),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn watched(
    State(app): State<App>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = kept_address("/watched", number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/watched");
    };
    match app.watched(&session, number).await {
        Ok(topics) => render(
            &guard,
            StatusCode::OK,
            html::page(&chrome, "Watched", &html::watched_page(&topics)),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn notifications(
    State(app): State<App>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = kept_address("/notifications", number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/notifications");
    };
    let notices = match app.notifications(&session, number).await {
        Ok(notices) => notices,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    };
    match app.followed_tags(&session).await {
        Ok(tags) => render(
            &guard,
            StatusCode::OK,
            html::page(
                &chrome,
                "Notifications",
                &html::notifications_page(&notices, &tags, &chrome),
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn read_notification(
    State(app): State<App>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/notifications").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/notifications");
    };
    match app.mark_read(&session, &id).await {
        Ok(()) => went(&guard, &None, "/notifications"),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(
                &chrome,
                "The notice was not marked read",
                &error.to_string(),
            ),
        ),
    }
}

async fn archive_month(
    State(app): State<App>,
    Path((year, month)): Path<(i32, u8)>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = archive_address(year, month, number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let session = session_of(&headers);
    match app
        .archive_month(year, month, number, session.as_deref())
        .await
    {
        Ok(Some(topics)) => render(
            &guard,
            StatusCode::OK,
            html::page(
                &chrome,
                &html::month_title(year, month),
                &html::archive_month_page(year, month, &topics),
            ),
        ),
        Ok(None) => render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such month",
                "This board holds no month at that address.",
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn tag(
    State(app): State<App>,
    Path(name): Path<String>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let number = page_of(query.page.as_deref());
    let address = tag_address(&name, number);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let session = session_of(&headers);
    let record = match app.tag(&name, session.as_deref()).await {
        Ok(record) => record,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    };
    match app.tag_topics(&name, number, session.as_deref()).await {
        Ok(topics) => render(
            &guard,
            StatusCode::OK,
            html::page(
                &chrome,
                &record.slug,
                &html::tag_page(&record, &topics, &chrome),
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
}

async fn follow_tag(
    State(app): State<App>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = tag_address(&name, 1);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    match app.follow_tag(&session, &name).await {
        Ok(()) => went(&guard, &None, &address),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(&chrome, "The tag was not taken up", &error.to_string()),
        ),
    }
}

async fn unfollow_tag(
    State(app): State<App>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = tag_address(&name, 1);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    match app.unfollow_tag(&session, &name).await {
        Ok(()) => went(&guard, &None, &address),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(&chrome, "The tag was not left", &error.to_string()),
        ),
    }
}

#[derive(Deserialize)]
pub struct DescribeForm {
    token: Option<String>,
    description: Option<String>,
    means: Option<String>,
}

async fn describe_tag(
    State(app): State<App>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Form(form): Form<DescribeForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = tag_address(&name, 1);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, &address);
    };
    let words = nonempty(form.description.as_deref());
    let means = nonempty(form.means.as_deref());
    match app
        .describe_tag(&session, &name, words.as_deref(), means.as_deref())
        .await
    {
        Ok(()) => went(&guard, &None, &address),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::message(&chrome, "The words were not kept", &error.to_string()),
        ),
    }
}

fn nonempty(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|words| !words.is_empty())
        .map(|words| words.to_owned())
}

async fn profile(
    State(app): State<App>,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = profile_address(&username);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    let Some(profile) = (match app.profile(&username).await {
        Ok(profile) => profile,
        Err(error) => {
            return render(
                &guard,
                StatusCode::SERVICE_UNAVAILABLE,
                unavailable(&chrome, &error),
            );
        }
    }) else {
        return render(
            &guard,
            StatusCode::NOT_FOUND,
            html::message(
                &chrome,
                "No such account",
                "This board has no account at that name.",
            ),
        );
    };
    let view = html::ProfileView {
        own: chrome.account_name() == Some(profile.username.as_str()),
        has_avatar: app.avatar(&username).await.is_some(),
    };
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            &profile.username,
            &html::profile_page(&chrome, &profile, &view, None),
        ),
    )
}

async fn avatar(State(app): State<App>, Path(username): Path<String>) -> Response {
    match app.avatar(&username).await {
        Some(picture) => (
            [
                (header::CONTENT_TYPE, picture.content_type().to_owned()),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
            ],
            picture.bytes().to_vec(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct BioForm {
    token: Option<String>,
    bio: Option<String>,
}

async fn change_bio(
    State(app): State<App>,
    Path(username): Path<String>,
    headers: HeaderMap,
    Form(form): Form<BioForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = profile_address(&username);
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return render(&guard, StatusCode::FORBIDDEN, not_yours(&chrome));
    };
    if chrome.account_name() != Some(username.as_str()) {
        return render(&guard, StatusCode::FORBIDDEN, not_yours(&chrome));
    }
    let words = form
        .bio
        .as_deref()
        .map(str::trim)
        .filter(|words| !words.is_empty())
        .map(|words| words.to_owned());
    match app.update_bio(&session, words.as_deref()).await {
        Ok(_) => (StatusCode::SEE_OTHER, [(header::LOCATION, address)]).into_response(),
        Err(error) => match app.profile(&username).await {
            Ok(Some(profile)) => render(
                &guard,
                StatusCode::UNPROCESSABLE_ENTITY,
                html::page(
                    &chrome,
                    &profile.username,
                    &html::profile_page(
                        &chrome,
                        &profile,
                        &html::ProfileView {
                            own: true,
                            has_avatar: app.avatar(&username).await.is_some(),
                        },
                        Some(&error.to_string()),
                    ),
                ),
            ),
            _ => render(
                &guard,
                StatusCode::UNPROCESSABLE_ENTITY,
                html::message(&chrome, "The words were not kept", &error.to_string()),
            ),
        },
    }
}

async fn password_form(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/password").await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Change password",
            &html::password_page(&chrome, None),
        ),
    )
}

#[derive(Deserialize)]
pub struct PasswordForm {
    token: Option<String>,
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<PasswordForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/password").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/settings/password");
    };
    match app
        .change_password(&session, &form.current_password, &form.new_password)
        .await
    {
        Ok(cookie) => went(&guard, &cookie, &home_of(chrome.account_name())),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Change password",
                &html::password_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

async fn email_form(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/email").await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Change the address",
            &html::email_page(&chrome, None),
        ),
    )
}

#[derive(Deserialize)]
pub struct EmailForm {
    token: Option<String>,
    email: String,
}

async fn request_email(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<EmailForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/email").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/settings/email");
    };
    match app.request_email_change(&session, &form.email).await {
        Ok(()) => render(&guard, StatusCode::OK, mailed(&chrome)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Change the address",
                &html::email_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct CodeForm {
    token: Option<String>,
    code: String,
}

async fn confirm_email(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<CodeForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/email").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    match app.confirm_email(&form.code).await {
        Ok(user) => went(&guard, &None, &home_of(Some(&user.username))),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Change the address",
                &html::email_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

async fn deregister_form(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/deregister").await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Leave the board",
            &html::deregister_page(&chrome, None),
        ),
    )
}

async fn deregister(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/settings/deregister").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(session) = session_of(&headers) else {
        return sign_in_first(&guard, "/settings/deregister");
    };
    match app.deregister(&session).await {
        Ok(cookie) => went(&guard, &cookie, "/"),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Leave the board",
                &html::deregister_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct TokenForm {
    token: Option<String>,
}

async fn forgot_form(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/forgot").await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Recover a password",
            &html::forgot_page(&chrome, None),
        ),
    )
}

async fn request_reset(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<EmailForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/forgot").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    match app.request_reset(&form.email).await {
        Ok(()) => render(&guard, StatusCode::OK, mailed(&chrome)),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Recover a password",
                &html::forgot_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

async fn reset_form(
    State(app): State<App>,
    Query(query): Query<CodeQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let code = query.code.unwrap_or_default();
    let address = format!("/forgot/confirm?code={}", client::encode_path(&code));
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Choose a new password",
            &html::forgot_confirm_page(&chrome, &code, None),
        ),
    )
}

#[derive(Deserialize)]
pub struct CodeQuery {
    code: Option<String>,
}

#[derive(Deserialize)]
pub struct NewPasswordForm {
    token: Option<String>,
    code: String,
    new_password: String,
}

async fn confirm_reset(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<NewPasswordForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/forgot/confirm").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    match app.reset_password(&form.code, &form.new_password).await {
        Ok(()) => went(&guard, &None, "/sign-in"),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Choose a new password",
                &html::forgot_confirm_page(&chrome, &form.code, Some(&error.to_string())),
            ),
        ),
    }
}

async fn activate_form(
    State(app): State<App>,
    Query(query): Query<CodeQuery>,
    headers: HeaderMap,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let code = query.code.unwrap_or_default();
    let chrome = chrome_of(&guard, &app, &headers, theme, "/activate").await;
    render(
        &guard,
        StatusCode::OK,
        html::page(
            &chrome,
            "Activate an account",
            &html::activate_page(&chrome, &code, None),
        ),
    )
}

async fn activate(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<CodeForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/activate").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    match app.activate(&form.code).await {
        Ok(_) => render(
            &guard,
            StatusCode::OK,
            html::message(
                &chrome,
                "The account is active",
                "The account is active now. Sign in to use it.",
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Activate an account",
                &html::activate_page(&chrome, &form.code, Some(&error.to_string())),
            ),
        ),
    }
}

fn home_of(account: Option<&str>) -> String {
    match account {
        Some(name) => profile_address(name),
        None => "/".to_owned(),
    }
}

fn went(guard: &Guard, cookie: &Option<String>, address: &str) -> Response {
    match (guard.set_cookie(), cookie) {
        (Some(token), Some(session)) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, token),
                (header::SET_COOKIE, session.clone()),
                (header::LOCATION, address.to_owned()),
            ],
        )
            .into_response(),
        (Some(token), None) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, token),
                (header::LOCATION, address.to_owned()),
            ],
        )
            .into_response(),
        (None, Some(session)) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, session.clone()),
                (header::LOCATION, address.to_owned()),
            ],
        )
            .into_response(),
        (None, None) => (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, address.to_owned())],
        )
            .into_response(),
    }
}

fn mailed(chrome: &Chrome) -> String {
    html::message(
        chrome,
        "Check your mail",
        "The board sent a secret to that address. Put it in the form on the page.",
    )
}

fn sign_in_first(guard: &Guard, address: &str) -> Response {
    let location = format!("/sign-in?return_to={address}");
    match guard.set_cookie() {
        Some(cookie) => (
            StatusCode::SEE_OTHER,
            [(header::SET_COOKIE, cookie), (header::LOCATION, location)],
        )
            .into_response(),
        None => (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response(),
    }
}

fn not_yours(chrome: &Chrome) -> String {
    html::message(
        chrome,
        "Not your page",
        "Only the account itself can change those words.",
    )
}

async fn register_form(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/register").await;
    render(
        &guard,
        StatusCode::OK,
        html::page(&chrome, "Register", &html::register_page(&chrome, None)),
    )
}

#[derive(Deserialize)]
pub struct RegisterForm {
    token: Option<String>,
    username: String,
    email: String,
    password: String,
    invitation: Option<String>,
}

async fn register(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<RegisterForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/register").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let mut body = client::RegisterBody::new(&form.username, &form.email, &form.password);
    if let Some(code) = form
        .invitation
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
    {
        body = body.invitation(code);
    }
    match app.register(&body).await {
        Ok(session) => signed_in(&session, "/"),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Register",
                &html::register_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

async fn sign_in_form(
    State(app): State<App>,
    headers: HeaderMap,
    Query(query): Query<ReturnQuery>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = return_to(query.return_to.as_deref());
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    render(
        &guard,
        StatusCode::OK,
        html::page(&chrome, "Sign in", &html::sign_in_page(&chrome, None)),
    )
}

#[derive(Deserialize)]
pub struct ReturnQuery {
    return_to: Option<String>,
}

#[derive(Deserialize)]
pub struct SignInForm {
    token: Option<String>,
    username: String,
    password: String,
    return_to: Option<String>,
}

async fn sign_in(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<SignInForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let address = return_to(form.return_to.as_deref());
    let chrome = chrome_of(&guard, &app, &headers, theme, &address).await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    match app.sign_in(&form.username, &form.password).await {
        Ok(session) => signed_in(&session, &address),
        Err(error) => render(
            &guard,
            StatusCode::UNPROCESSABLE_ENTITY,
            html::page(
                &chrome,
                "Sign in",
                &html::sign_in_page(&chrome, Some(&error.to_string())),
            ),
        ),
    }
}

#[derive(Deserialize)]
pub struct SignOutForm {
    token: Option<String>,
}

async fn sign_out(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<SignOutForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/").await;
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let gone = match session_of(&headers) {
        Some(session) => app.sign_out(&session).await,
        None => None,
    };
    let cookie = gone
        .unwrap_or_else(|| format!("{SESSION_COOKIE}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"));
    (
        StatusCode::SEE_OTHER,
        [
            (header::SET_COOKIE, cookie),
            (header::LOCATION, "/".to_owned()),
        ],
    )
        .into_response()
}

fn signed_in(session: &client::Session, address: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::SET_COOKIE, session.cookie().to_owned()),
            (header::LOCATION, address.to_owned()),
        ],
    )
        .into_response()
}

fn expired(chrome: &Chrome) -> String {
    html::message(
        chrome,
        "The form has expired",
        "Send the form again from the page it came from.",
    )
}

fn unavailable(chrome: &Chrome, error: &client::ClientError) -> String {
    html::message(chrome, "The board is not answering", &error.to_string())
}

async fn chrome_of(
    guard: &Guard,
    app: &App,
    headers: &HeaderMap,
    theme: Theme,
    return_to: &str,
) -> Chrome {
    let account = match session_of(headers) {
        Some(session) => app.account(&session).await,
        None => None,
    };
    let chrome = Chrome::new(theme, guard.token()).return_to(return_to);
    match account {
        Some(account) => chrome.account(&account.username, &account.role),
        None => chrome,
    }
}

fn session_of(headers: &HeaderMap) -> Option<String> {
    cookie_of(headers, SESSION_COOKIE)
}

fn topic_address(id: &str, page: u32) -> String {
    if page <= 1 {
        format!("/topics/{}", client::encode_path(id))
    } else {
        format!("/topics/{}?page={}", client::encode_path(id), page)
    }
}

fn profile_address(username: &str) -> String {
    format!("/u/{}", client::encode_path(username))
}

fn tag_address(name: &str, page: u32) -> String {
    if page <= 1 {
        format!("/tags/{}", client::encode_path(name))
    } else {
        format!("/tags/{}?page={}", client::encode_path(name), page)
    }
}

fn archive_address(year: i32, month: u8, page: u32) -> String {
    let address = html::archive_month_address(year, month);
    if page <= 1 {
        address
    } else {
        format!("{address}?page={page}")
    }
}

fn kept_address(address: &str, page: u32) -> String {
    if page <= 1 {
        address.to_owned()
    } else {
        format!("{address}?page={page}")
    }
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

fn render(guard: &Guard, status: StatusCode, body: String) -> Response {
    match guard.set_cookie() {
        Some(cookie) => (status, [(header::SET_COOKIE, cookie)], Html(body)).into_response(),
        None => (status, Html(body)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ThemeForm {
    theme: String,
    return_to: Option<String>,
    token: Option<String>,
}

async fn set_theme(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<ThemeForm>,
) -> Response {
    let guard = Guard::new(&headers);
    let chrome = chrome_of(&guard, &app, &headers, app.default_theme(), "/")
        .await
        .return_to(&return_to(form.return_to.as_deref()));
    if !guard.allows(form.token.as_deref()) {
        return render(&guard, StatusCode::FORBIDDEN, expired(&chrome));
    }
    let Some(theme) = Theme::parse(&form.theme) else {
        let known: Vec<&str> = Theme::ALL.iter().map(|theme| theme.name()).collect();
        return render(
            &guard,
            StatusCode::BAD_REQUEST,
            html::message(
                &chrome,
                "No such theme",
                &format!("Choose one of: {}.", known.join(", ")),
            ),
        );
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

async fn not_found(State(app): State<App>, headers: HeaderMap) -> Response {
    let guard = Guard::new(&headers);
    let theme = theme_of(&headers, &app);
    let chrome = chrome_of(&guard, &app, &headers, theme, "/").await;
    render(
        &guard,
        StatusCode::NOT_FOUND,
        html::message(&chrome, "Nothing here", "This page does not exist."),
    )
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
    let chosen = cookie_of(headers, "theme");
    app.theme_for(chosen.as_deref())
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
