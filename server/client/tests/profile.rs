use client::{ApiClient, ClientError, Profile};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn requests(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    fn last(&self) -> String {
        self.requests().last().cloned().unwrap_or_default()
    }
}

async fn board(response: &'static str) -> (String, Log) {
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
                sink.lock().unwrap().push(request);
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    (format!("http://{address}"), log)
}

fn answer(status: &'static str, headers: &[&'static str], body: &'static str) -> &'static str {
    let mut response = format!("HTTP/1.1 {status}\r\n");
    for header in headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    response.push_str(&format!("content-length: {}\r\n", body.len()));
    response.push_str("connection: close\r\n\r\n");
    let leaked: &'static mut String = Box::leak(Box::new(response + body));
    leaked
}

const PROFILE: &str = "{\"id\":\"7f2c\",\"username\":\"alice\",\"bio\":\"hello\",\
                       \"score\":12,\"role\":\"user\"}";
const NO_BIO: &str = "{\"id\":\"7f2c\",\"username\":\"alice\",\"bio\":null,\
                      \"score\":0,\"role\":\"user\"}";

#[tokio::test]
async fn a_profile_is_read_by_the_name_of_its_account() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        PROFILE,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let profile = client.profile("alice").await.unwrap();
    assert_eq!(
        profile,
        Profile {
            id: "7f2c".to_owned(),
            username: "alice".to_owned(),
            bio: Some("hello".to_owned()),
            score: 12,
            role: "user".to_owned(),
        }
    );
    let request = log.last();
    assert!(request.starts_with("GET /api/users/alice "), "{request}");
}

#[tokio::test]
async fn a_profile_without_words_on_itself_comes_back_empty() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        NO_BIO,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(client.profile("alice").await.unwrap().bio, None);
}

#[tokio::test]
async fn an_odd_name_stays_inside_one_step_of_the_address() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        PROFILE,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    client.profile("a/b?c d").await.unwrap();
    assert!(
        log.last().starts_with("GET /api/users/a%2Fb%3Fc%20d "),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn a_name_nobody_owns_is_refused_with_the_reason() {
    let (base, _) = board(answer(
        "404 Not Found",
        &["content-type: application/json"],
        "{\"error\":\"user not found\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.profile("nobody").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 404);
            assert_eq!(reason, "user not found");
        }
        other => panic!("a name nobody owns must be refused: {other:?}"),
    }
}

#[tokio::test]
async fn an_unreadable_profile_is_an_error() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        "not json",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.profile("alice").await {
        Err(ClientError::Detail(_)) => {}
        other => panic!("an unreadable answer must be an error: {other:?}"),
    }
}

#[tokio::test]
async fn a_bio_is_carried_to_the_board_with_the_session() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        PROFILE,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let profile = client
        .update_bio("tok9", Some("Rust and forums."))
        .await
        .unwrap();
    assert_eq!(profile.bio.as_deref(), Some("hello"));
    let request = log.last();
    assert!(request.starts_with("PATCH /api/me/bio "), "{request}");
    assert!(request.contains("cookie: session=tok9"), "{request}");
    assert!(
        request.contains("{\"bio\":\"Rust and forums.\"}"),
        "{request}"
    );
}

#[tokio::test]
async fn an_empty_bio_takes_the_one_on_the_board_away() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        NO_BIO,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(client.update_bio("tok9", None).await.unwrap().bio, None);
    let request = log.last();
    assert!(request.contains("{\"bio\":null}"), "{request}");
}

#[tokio::test]
async fn a_bio_the_board_refuses_keeps_its_reason() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        &["content-type: application/json"],
        "{\"error\":\"bio too long\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.update_bio("tok9", Some("too long")).await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 422);
            assert_eq!(reason, "bio too long");
        }
        other => panic!("a refused bio must keep its reason: {other:?}"),
    }
}

#[tokio::test]
async fn an_avatar_comes_back_with_its_kind_and_its_bytes() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: image/png"],
        "image-bytes-here",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let avatar = client.avatar("alice").await.unwrap().unwrap();
    assert_eq!(avatar.content_type(), "image/png");
    assert_eq!(avatar.bytes(), b"image-bytes-here");
    let request = log.last();
    assert!(
        request.starts_with("GET /api/users/alice/avatar "),
        "{request}"
    );
}

#[tokio::test]
async fn an_account_without_an_avatar_has_none() {
    let (base, _) = board(answer(
        "404 Not Found",
        &["content-type: application/json"],
        "{\"error\":\"no avatar\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.avatar("alice").await.unwrap().is_none());
}

#[tokio::test]
async fn an_avatar_of_an_odd_name_stays_inside_one_step_of_the_address() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: image/png"],
        "image-bytes-here",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    client.avatar("a/b").await.unwrap();
    assert!(
        log.last().starts_with("GET /api/users/a%2Fb/avatar "),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn an_avatar_the_board_cannot_answer_is_an_error() {
    let (base, _) = board(answer(
        "500 Internal Server Error",
        &["content-type: application/json"],
        "{\"error\":\"broken\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.avatar("alice").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 500);
            assert_eq!(reason, "broken");
        }
        other => panic!("a broken avatar answer must be an error: {other:?}"),
    }
}

#[tokio::test]
async fn an_avatar_without_a_kind_is_kept_as_a_stream_of_bytes() {
    let (base, _) = board(answer("200 OK", &[], "image-bytes-here")).await;
    let client = ApiClient::new(&base).unwrap();
    let avatar = client.avatar("alice").await.unwrap().unwrap();
    assert_eq!(avatar.content_type(), "application/octet-stream");
}
