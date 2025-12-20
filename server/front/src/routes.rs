use axum::{
    Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;

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
        .route("/topics/{id}", get(topic))
        .route("/search", get(search))
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
    let Some(subject) = (match app.subject(&id).await {
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
    match app.comments(&id, number).await {
        Ok(comments) => render(
            &guard,
            StatusCode::OK,
            html::page(
                &chrome,
                &subject.title,
                &html::subject_page(&subject, &comments),
            ),
        ),
        Err(error) => render(
            &guard,
            StatusCode::SERVICE_UNAVAILABLE,
            unavailable(&chrome, &error),
        ),
    }
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
    let address = search_address(&criteria);
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
        Some(account) => chrome.account(&account.username),
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

fn search_address(criteria: &client::Criteria) -> String {
    format!(
        "/search?q={}&scope={}&order={}",
        client::encode_path(criteria.query()),
        criteria.scope().label(),
        criteria.order().label()
    )
}

fn profile_address(username: &str) -> String {
    format!("/u/{}", client::encode_path(username))
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
