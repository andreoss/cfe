use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECTIONS: &str = "[]";
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
                    "/api/register" | "/api/sign-in" if mode == "taken" => (
                        "409 Conflict",
                        vec!["content-type: application/json"],
                        "{\"error\":\"username taken\"}",
                    ),
                    "/api/register" | "/api/sign-in" => (
                        "200 OK",
                        vec![
                            "content-type: application/json",
                            "set-cookie: session=tok9; Path=/; HttpOnly; SameSite=Strict",
                        ],
                        ACCOUNT,
                    ),
                    "/api/sign-out" => (
                        "200 OK",
                        vec!["set-cookie: session=; Path=/; Max-Age=0; HttpOnly"],
                        "",
                    ),
                    "/api/me" => ("200 OK", vec!["content-type: application/json"], ACCOUNT),
                    _ => ("200 OK", vec!["content-type: application/json"], SECTIONS),
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
async fn the_register_page_is_a_form_with_a_token() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/register"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Register</h2>"), "{page}");
    assert!(page.contains("method=\"post\" action=\"/register\""));
    assert!(page.contains("id=\"username\" name=\"username\" type=\"text\""));
    assert!(page.contains("id=\"email\" name=\"email\" type=\"email\""));
    assert!(page.contains("id=\"password\" name=\"password\" type=\"password\""));
    assert!(page.contains("type=\"hidden\" name=\"token\""));
    assert!(front::html::scripting_free(&page), "scripting: {page}");
}

#[tokio::test]
async fn a_registration_sets_the_session_cookie() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/register"))
        .header("cookie", cookies)
        .form(&[
            ("username", "alice"),
            ("email", "a@example.org"),
            ("password", "secret11"),
            ("invitation", ""),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|raw| raw.starts_with("session="))
        .expect("the session is set")
        .to_owned();
    assert!(cookie.starts_with("session=tok9"), "{cookie}");
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/"
    );
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("username"))
        .unwrap_or_default();
    assert!(sent.contains("\"username\":\"alice\""), "{sent}");
    assert!(sent.contains("\"email\":\"a@example.org\""), "{sent}");
    assert!(sent.contains("\"password\":\"secret11\""), "{sent}");
    assert!(!sent.contains("invitation"), "{sent}");
}

#[tokio::test]
async fn a_registration_the_board_refuses_says_why() {
    let (board_url, _) = board("taken").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/register"))
        .header("cookie", cookies)
        .form(&[
            ("username", "alice"),
            ("email", "a@example.org"),
            ("password", "secret11"),
            ("token", &token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    let page = response.text().await.unwrap();
    assert!(page.contains("username taken"), "{page}");
    assert!(page.contains("method=\"post\" action=\"/register\""));
}

#[tokio::test]
async fn a_form_without_a_token_is_refused() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let response = http()
        .post(format!("{base}/register"))
        .form(&[
            ("username", "alice"),
            ("email", "a@example.org"),
            ("password", "secret11"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("The form has expired")
    );
    assert!(!log.requests().iter().any(|r| r.contains("/api/register")));
}

#[tokio::test]
async fn the_sign_in_page_keeps_the_address_it_came_from() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/sign-in?return_to=/sections/general"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("method=\"post\" action=\"/sign-in\""));
    assert!(page.contains("value=\"/sections/general\""));
    assert!(front::html::scripting_free(&page), "scripting: {page}");
}

#[tokio::test]
async fn a_sign_in_sets_the_session_cookie_and_returns_to_the_page() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/sign-in"))
        .header("cookie", cookies)
        .form(&[
            ("username", "alice"),
            ("password", "secret11"),
            ("return_to", "/sections/general"),
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
        "/sections/general"
    );
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|raw| raw.starts_with("session="))
        .expect("the session is set")
        .to_owned();
    assert!(cookie.starts_with("session=tok9"), "{cookie}");
}

#[tokio::test]
async fn a_signed_in_reader_is_named_in_the_masthead() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(&base)
        .header("cookie", "session=tok9")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("Signed in as"), "{page}");
    assert!(page.contains("href=\"/u/alice\""), "{page}");
    assert!(page.contains("method=\"post\" action=\"/sign-out\""));
    assert!(log.requests().iter().any(
        |request| request.contains("GET /api/me ") && request.contains("cookie: session=tok9")
    ));
}

#[tokio::test]
async fn a_reader_without_a_session_is_offered_the_door() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(&base)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<a href=\"/sign-in\">Sign in</a>"), "{page}");
    assert!(
        page.contains("<a href=\"/register\">register</a>"),
        "{page}"
    );
    assert!(!log.requests().iter().any(|r| r.contains("/api/me")));
}

#[tokio::test]
async fn a_sign_out_drops_the_session_cookie() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let token = token_of(&base).await.1;
    let response = http()
        .post(format!("{base}/sign-out"))
        .header("cookie", format!("session=tok9; token={token}"))
        .form(&[("token", token.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|raw| raw.starts_with("session="))
        .expect("the board's answer is relayed")
        .to_owned();
    assert!(cookie.contains("Max-Age=0"), "{cookie}");
    assert!(
        log.requests()
            .iter()
            .any(|request| request.contains("POST /api/sign-out")
                && request.contains("cookie: session=tok9"))
    );
}
