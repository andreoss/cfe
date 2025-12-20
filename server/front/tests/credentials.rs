use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ACCOUNT: &str = "{\"id\":\"7f2c\",\"username\":\"alice\",\"role\":\"user\"}";

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn requests(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    fn bodies(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| {
                request
                    .split("\r\n\r\n")
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect()
    }
}

async fn board(mode: &'static str) -> (String, Log) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let log = Log(Arc::new(Mutex::new(Vec::new())));
    let sink = log.0.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(_) => return,
            };
            let sink = sink.clone();
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 8192];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                sink.lock().unwrap().push(request.clone());
                let target = request.split_whitespace().nth(1).unwrap_or("/");
                let path = target.split('?').next().unwrap_or("");
                let (status, headers, body): (&str, Vec<&str>, &str) = match path {
                    "/api/me" if mode == "nobody" => {
                        ("200 OK", vec!["content-type: application/json"], "null")
                    }
                    "/api/me" => ("200 OK", vec!["content-type: application/json"], ACCOUNT),
                    "/api/me/password" if mode == "wrong" => (
                        "401 Unauthorized",
                        vec!["content-type: application/json"],
                        "{\"error\":\"wrong current password\"}",
                    ),
                    "/api/me/password" => (
                        "200 OK",
                        vec![
                            "content-type: application/json",
                            "set-cookie: session=new9; Path=/; HttpOnly; SameSite=Strict",
                        ],
                        ACCOUNT,
                    ),
                    "/api/me/email" if mode == "taken" => (
                        "409 Conflict",
                        vec!["content-type: application/json"],
                        "{\"error\":\"address taken\"}",
                    ),
                    "/api/me/email" => ("202 Accepted", vec![], ""),
                    "/api/me/email/confirm" if mode == "expired" => (
                        "422 Unprocessable Entity",
                        vec!["content-type: application/json"],
                        "{\"error\":\"invalid or expired code\"}",
                    ),
                    "/api/me/email/confirm" => {
                        ("200 OK", vec!["content-type: application/json"], ACCOUNT)
                    }
                    "/api/password-reset" => ("202 Accepted", vec![], ""),
                    "/api/password-reset/confirm" if mode == "expired" => (
                        "422 Unprocessable Entity",
                        vec!["content-type: application/json"],
                        "{\"error\":\"invalid or expired code\"}",
                    ),
                    "/api/password-reset/confirm" => ("204 No Content", vec![], ""),
                    "/api/activate" if mode == "expired" => (
                        "422 Unprocessable Entity",
                        vec!["content-type: application/json"],
                        "{\"error\":\"invalid or expired code\"}",
                    ),
                    "/api/activate" => ("200 OK", vec!["content-type: application/json"], ACCOUNT),
                    "/api/me/deregister" => (
                        "200 OK",
                        vec![
                            "content-type: application/json",
                            "set-cookie: session=; Path=/; Max-Age=0; HttpOnly",
                        ],
                        ACCOUNT,
                    ),
                    _ => ("200 OK", vec!["content-type: application/json"], "[]"),
                };
                let mut response = format!("HTTP/1.1 {status}\r\n").to_owned();
                for header in headers {
                    response.push_str(header);
                    response.push_str("\r\n");
                }
                response.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
                response.push_str(body);
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    (format!("http://{address}"), log)
}

async fn front(server_url: &str) -> String {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(front::config::SERVER_URL.to_owned(), server_url.to_owned());
    let config = Config::from_vars(&vars);
    let app = front::App::new(&config).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, routes::router(app)).await;
    });
    format!("http://{address}")
}

async fn token_of(base: &str) -> (String, String) {
    let response = http().get(base).send().await.unwrap();
    let token = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(';').next())
        .and_then(|pair| pair.strip_prefix("token="))
        .expect("a page answers with a token")
        .to_owned();
    response.text().await.unwrap();
    (format!("token={token}"), token)
}

#[tokio::test]
async fn the_password_page_is_a_form_with_a_token() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/settings/password"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Change password</h2>"), "{page}");
    assert!(page.contains("action=\"/settings/password\""), "{page}");
    assert!(
        page.contains("id=\"current_password\" name=\"current_password\" type=\"password\""),
        "{page}"
    );
    assert!(
        page.contains("id=\"new_password\" name=\"new_password\""),
        "{page}"
    );
    assert!(page.contains("type=\"hidden\" name=\"token\""), "{page}");
    assert!(front::html::scripting_free(&page), "scripting: {page}");
}

#[tokio::test]
async fn a_new_password_is_carried_to_the_board_and_the_session_is_replaced() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/password"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[
            ("current_password", "secret11"),
            ("new_password", "secret22"),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/u/alice"
    );
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|raw| raw.starts_with("session="))
        .expect("the new session is relayed")
        .to_owned();
    assert!(cookie.starts_with("session=new9"), "{cookie}");
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("password"))
        .unwrap_or_default();
    assert!(sent.contains("\"current_password\":\"secret11\""), "{sent}");
    assert!(sent.contains("\"new_password\":\"secret22\""), "{sent}");
    assert!(
        log.requests()
            .iter()
            .any(|request| request.contains("POST /api/me/password")
                && request.contains("cookie: session=tok9")),
        "{:?}",
        log.requests()
    );
}

#[tokio::test]
async fn a_password_the_board_refuses_comes_back_on_the_form() {
    let (board_url, _) = board("wrong").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/password"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[
            ("current_password", "secret11"),
            ("new_password", "secret22"),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    let page = response.text().await.unwrap();
    assert!(page.contains("wrong current password"), "{page}");
    assert!(page.contains("action=\"/settings/password\""), "{page}");
}

#[tokio::test]
async fn a_password_change_without_a_session_is_sent_to_the_door() {
    let (board_url, log) = board("nobody").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/password"))
        .header("cookie", cookies)
        .form(&[
            ("current_password", "secret11"),
            ("new_password", "secret22"),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("/sign-in?return_to=/settings/password")
    );
    assert!(
        !log.requests()
            .iter()
            .any(|r| r.contains("/api/me/password"))
    );
}

#[tokio::test]
async fn the_address_page_asks_for_an_address_and_for_the_secret() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/settings/email"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/settings/email\""), "{page}");
    assert!(
        page.contains("id=\"email\" name=\"email\" type=\"email\""),
        "{page}"
    );
    assert!(
        page.contains("action=\"/settings/email/confirm\""),
        "{page}"
    );
    assert!(page.contains("id=\"code\" name=\"code\""), "{page}");
    assert!(front::html::scripting_free(&page), "scripting: {page}");
}

#[tokio::test]
async fn a_new_address_is_carried_to_the_board() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/email"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("email", "b@example.org"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(
        response.text().await.unwrap().contains("Check your mail"),
        "the reader is told to look at their mail"
    );
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("email"))
        .unwrap_or_default();
    assert!(sent.contains("\"email\":\"b@example.org\""), "{sent}");
    assert!(
        log.requests()
            .iter()
            .any(|request| request.contains("POST /api/me/email")
                && request.contains("cookie: session=tok9"))
    );
}

#[tokio::test]
async fn an_address_somebody_holds_says_so_on_the_form() {
    let (board_url, _) = board("taken").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/email"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("email", "b@example.org"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    assert!(
        response.text().await.unwrap().contains("address taken"),
        "the board's words come back"
    );
}

#[tokio::test]
async fn the_secret_confirms_the_new_address() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/email/confirm"))
        .header("cookie", cookies)
        .form(&[("code", "code-1"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/u/alice"
    );
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("code"))
        .unwrap_or_default();
    assert!(sent.contains("\"code\":\"code-1\""), "{sent}");
}

#[tokio::test]
async fn the_recovery_page_asks_for_the_address_on_file() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/forgot"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Recover a password</h2>"), "{page}");
    assert!(page.contains("action=\"/forgot\""), "{page}");
    assert!(
        page.contains("id=\"email\" name=\"email\" type=\"email\""),
        "{page}"
    );
    assert!(front::html::scripting_free(&page), "scripting: {page}");
}

#[tokio::test]
async fn a_recovery_is_asked_for_and_the_reader_is_told_to_look_at_their_mail() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/forgot"))
        .header("cookie", cookies)
        .form(&[("email", "a@example.org"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(response.text().await.unwrap().contains("Check your mail"));
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("email"))
        .unwrap_or_default();
    assert!(sent.contains("\"email\":\"a@example.org\""), "{sent}");
}

#[tokio::test]
async fn the_secret_from_the_mail_is_kept_on_the_new_password_page() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/forgot/confirm?code=code-1"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/forgot/confirm\""), "{page}");
    assert!(page.contains("value=\"code-1\""), "{page}");
    assert!(
        page.contains("id=\"new_password\" name=\"new_password\""),
        "{page}"
    );
}

#[tokio::test]
async fn a_recovery_ends_at_the_door() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/forgot/confirm"))
        .header("cookie", cookies)
        .form(&[
            ("code", "code-1"),
            ("new_password", "secret22"),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/sign-in"
    );
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("new_password"))
        .unwrap_or_default();
    assert!(sent.contains("\"code\":\"code-1\""), "{sent}");
    assert!(sent.contains("\"new_password\":\"secret22\""), "{sent}");
}

#[tokio::test]
async fn a_secret_nobody_knows_comes_back_on_the_recovery_page() {
    let (board_url, _) = board("expired").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/forgot/confirm"))
        .header("cookie", cookies)
        .form(&[
            ("code", "code-1"),
            ("new_password", "secret22"),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    let page = response.text().await.unwrap();
    assert!(page.contains("invalid or expired code"), "{page}");
    assert!(page.contains("value=\"code-1\""), "{page}");
}

#[tokio::test]
async fn an_account_is_activated_with_the_secret_from_the_mail() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/activate?code=code-1"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/activate\""), "{page}");
    assert!(page.contains("value=\"code-1\""), "{page}");
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/activate"))
        .header("cookie", cookies)
        .form(&[("code", "code-1"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("The account is active")
    );
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("code"))
        .unwrap_or_default();
    assert!(sent.contains("\"code\":\"code-1\""), "{sent}");
}

#[tokio::test]
async fn a_deregistration_drops_the_session_cookie() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/settings/deregister"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/settings/deregister\""), "{page}");
    assert!(page.contains("type=\"hidden\" name=\"token\""), "{page}");
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/settings/deregister"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("token", token.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/"
    );
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|raw| raw.starts_with("session="))
        .expect("the dropped session is relayed")
        .to_owned();
    assert!(cookie.contains("Max-Age=0"), "{cookie}");
    assert!(
        log.requests()
            .iter()
            .any(|request| request.contains("POST /api/me/deregister")
                && request.contains("cookie: session=tok9"))
    );
}

#[tokio::test]
async fn a_form_without_a_token_is_refused_before_the_board_is_asked() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    for address in [
        "/settings/password",
        "/settings/email",
        "/settings/deregister",
        "/forgot",
        "/forgot/confirm",
        "/activate",
    ] {
        let response = http()
            .post(format!("{base}{address}"))
            .header("cookie", "session=tok9")
            .form(&[
                ("current_password", "secret11"),
                ("new_password", "secret22"),
                ("email", "a@example.org"),
                ("code", "code-1"),
            ])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403, "{address}");
        assert!(
            response
                .text()
                .await
                .unwrap()
                .contains("The form has expired"),
            "{address}"
        );
    }
    assert!(
        !log.requests()
            .iter()
            .any(|request| request.contains("/api/me/password")
                || request.contains("/api/me/email")
                || request.contains("/api/me/deregister")
                || request.contains("/api/password-reset")
                || request.contains("/api/activate"))
    );
}
