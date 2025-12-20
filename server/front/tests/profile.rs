use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ALICE: &str = "{\"id\":\"7f2c\",\"username\":\"alice\",\"bio\":\"Rust and forums.\",\
                     \"score\":12,\"role\":\"user\"}";
const BOB: &str =
    "{\"id\":\"9d1e\",\"username\":\"bob\",\"bio\":null,\"score\":0,\"role\":\"user\"}";

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
                    "/api/me" if mode == "other" => (
                        "200 OK",
                        vec!["content-type: application/json"],
                        "{\"id\":\"9d1e\",\"username\":\"bob\",\"role\":\"user\"}",
                    ),
                    "/api/me" => (
                        "200 OK",
                        vec!["content-type: application/json"],
                        "{\"id\":\"7f2c\",\"username\":\"alice\",\"role\":\"user\"}",
                    ),
                    "/api/users/nobody" => (
                        "404 Not Found",
                        vec!["content-type: application/json"],
                        "{\"error\":\"user not found\"}",
                    ),
                    "/api/users/bob" => ("200 OK", vec!["content-type: application/json"], BOB),
                    "/api/me/bio" if mode == "long" => (
                        "422 Unprocessable Entity",
                        vec!["content-type: application/json"],
                        "{\"error\":\"bio too long\"}",
                    ),
                    "/api/me/bio" => ("200 OK", vec!["content-type: application/json"], ALICE),
                    "/api/users/alice/avatar" if mode == "no-avatar" => (
                        "404 Not Found",
                        vec!["content-type: application/json"],
                        "{\"error\":\"no avatar\"}",
                    ),
                    "/api/users/alice/avatar" => (
                        "200 OK",
                        vec!["content-type: image/png"],
                        "image-bytes-here",
                    ),
                    _ if path.starts_with("/api/users/") => {
                        ("200 OK", vec!["content-type: application/json"], ALICE)
                    }
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
async fn the_profile_page_carries_the_name_the_score_and_the_bio() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/u/alice"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>alice</h2>"), "{page}");
    assert!(page.contains("Rust and forums."), "{page}");
    assert!(page.contains("12"), "{page}");
    assert!(front::html::scripting_free(&page), "scripting: {page}");
}

#[tokio::test]
async fn an_account_nobody_owns_is_not_found() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let response = http().get(format!("{base}/u/nobody")).send().await.unwrap();
    assert_eq!(response.status(), 404);
    assert!(response.text().await.unwrap().contains("No such account"));
}

#[tokio::test]
async fn a_profile_whose_words_are_markup_keeps_them_as_words() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/u/alice"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn the_avatar_of_an_account_is_served_with_its_kind() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/u/alice/avatar"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/png"
    );
    assert_eq!(
        response.bytes().await.unwrap().as_ref(),
        b"image-bytes-here"
    );
}

#[tokio::test]
async fn an_account_without_an_avatar_has_no_picture() {
    let (board_url, _) = board("no-avatar").await;
    let base = front(&board_url).await;
    let missing = http()
        .get(format!("{base}/u/alice/avatar"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    let page = http()
        .get(format!("{base}/u/alice"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains("class=\"avatar\""), "{page}");
}

#[tokio::test]
async fn a_reader_sees_a_form_for_their_own_words() {
    let (board_url, _) = board("ok").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/u/alice"))
        .header("cookie", "session=tok9")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/u/alice/bio\""), "{page}");
    assert!(page.contains("id=\"bio\" name=\"bio\""), "{page}");
    assert!(page.contains("type=\"hidden\" name=\"token\""), "{page}");
    assert!(page.contains("Rust and forums."), "{page}");
}

#[tokio::test]
async fn the_words_of_another_reader_come_without_a_form() {
    let (board_url, _) = board("other").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/u/alice"))
        .header("cookie", "session=tok9")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains("action=\"/u/alice/bio\""), "{page}");
    assert!(page.contains("Rust and forums."), "{page}");
}

#[tokio::test]
async fn a_reader_without_a_session_sees_no_form() {
    let (board_url, _) = board("nobody").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/u/alice"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains("action=\"/u/alice/bio\""), "{page}");
}

#[tokio::test]
async fn a_bio_is_carried_to_the_board_with_the_session_and_returns() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/u/alice/bio"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("bio", "Rust and forums."), ("token", &token)])
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
        .find(|body| body.contains("bio"))
        .unwrap_or_default();
    assert!(sent.contains("\"bio\":\"Rust and forums.\""), "{sent}");
    assert!(
        log.requests()
            .iter()
            .any(|request| request.contains("PATCH /api/me/bio")
                && request.contains("cookie: session=tok9")),
        "{:?}",
        log.requests()
    );
}

#[tokio::test]
async fn an_empty_bio_takes_the_words_away() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/u/alice/bio"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("bio", "   "), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    let sent = log
        .bodies()
        .into_iter()
        .find(|body| body.contains("bio"))
        .unwrap_or_default();
    assert!(sent.contains("\"bio\":null"), "{sent}");
}

#[tokio::test]
async fn a_bio_the_board_refuses_comes_back_on_the_form() {
    let (board_url, _) = board("long").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/u/alice/bio"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("bio", "too long"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    let page = response.text().await.unwrap();
    assert!(page.contains("bio too long"), "{page}");
    assert!(page.contains("action=\"/u/alice/bio\""), "{page}");
}

#[tokio::test]
async fn a_form_without_a_token_is_refused() {
    let (board_url, log) = board("ok").await;
    let base = front(&board_url).await;
    let response = http()
        .post(format!("{base}/u/alice/bio"))
        .header("cookie", "session=tok9")
        .form(&[("bio", "hello")])
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
    assert!(!log.requests().iter().any(|r| r.contains("/api/me/bio")));
}

#[tokio::test]
async fn the_words_of_another_reader_cannot_be_changed() {
    let (board_url, log) = board("other").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/u/alice/bio"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("bio", "mine now"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert!(response.text().await.unwrap().contains("Not your page"));
    assert!(!log.requests().iter().any(|r| r.contains("/api/me/bio")));
}

#[tokio::test]
async fn a_stranger_without_a_session_cannot_change_words() {
    let (board_url, log) = board("nobody").await;
    let base = front(&board_url).await;
    let (cookies, token) = token_of(&base).await;
    let response = http()
        .post(format!("{base}/u/alice/bio"))
        .header("cookie", format!("{cookies}; session=tok9"))
        .form(&[("bio", "mine now"), ("token", &token)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert!(!log.requests().iter().any(|r| r.contains("/api/me/bio")));
}
