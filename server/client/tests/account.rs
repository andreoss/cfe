use client::{ApiClient, ClientError, User};
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

const ACCOUNT: &str = "{\"id\":\"7f2c\",\"username\":\"alice\",\"role\":\"user\"}";

#[tokio::test]
async fn a_registration_asks_the_board_for_an_account() {
    let (base, log) = board(answer(
        "201 Created",
        &[
            "content-type: application/json",
            "set-cookie: session=abc123; Path=/; HttpOnly",
        ],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let session = client
        .register(&client::RegisterBody::new(
            "alice",
            "a@example.org",
            "secret11",
        ))
        .await
        .unwrap();
    assert_eq!(session.token(), "abc123");
    assert!(session.cookie().starts_with("session=abc123"));
    let request = log.last();
    assert!(request.starts_with("POST /api/register "), "{request}");
    assert!(request.contains("\"username\":\"alice\""), "{request}");
    assert!(request.contains("\"email\":\"a@example.org\""), "{request}");
    assert!(request.contains("\"password\":\"secret11\""), "{request}");
}

#[tokio::test]
async fn a_registration_may_carry_an_invitation() {
    let (base, log) = board(answer(
        "201 Created",
        &[
            "content-type: application/json",
            "set-cookie: session=abc123; Path=/",
        ],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    client
        .register(
            &client::RegisterBody::new("alice", "a@example.org", "secret11").invitation("code-1"),
        )
        .await
        .unwrap();
    assert!(
        log.last().contains("\"invitation\":\"code-1\""),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn a_sign_in_carries_the_credentials_and_keeps_the_session() {
    let (base, log) = board(answer(
        "200 OK",
        &[
            "content-type: application/json",
            "set-cookie: session=tok9; Path=/",
        ],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let session = client.sign_in("alice", "secret11").await.unwrap();
    assert_eq!(session.token(), "tok9");
    let request = log.last();
    assert!(request.starts_with("POST /api/sign-in "), "{request}");
    assert!(request.contains("\"username\":\"alice\""), "{request}");
    assert!(request.contains("\"password\":\"secret11\""), "{request}");
}

#[tokio::test]
async fn an_answer_without_a_session_is_an_error() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.sign_in("alice", "secret11").await {
        Err(ClientError::Detail(detail)) => assert!(detail.contains("session"), "{detail}"),
        other => panic!("an answer without a session must be refused: {other:?}"),
    }
}

#[tokio::test]
async fn a_sign_out_carries_the_session_cookie() {
    let (base, log) = board(answer("204 No Content", &[], "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.sign_out("tok9").await.unwrap();
    let request = log.last();
    assert!(request.starts_with("POST /api/sign-out "), "{request}");
    assert!(request.contains("cookie: session=tok9"), "{request}");
}

#[tokio::test]
async fn the_account_of_a_signed_in_reader_comes_back() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let user = client.me(Some("tok9")).await.unwrap().unwrap();
    assert_eq!(
        user,
        User {
            id: "7f2c".to_owned(),
            username: "alice".to_owned(),
            role: "user".to_owned(),
        }
    );
    let request = log.last();
    assert!(request.starts_with("GET /api/me "), "{request}");
    assert!(request.contains("cookie: session=tok9"), "{request}");
}

#[tokio::test]
async fn a_reader_without_a_session_is_nobody() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        "null",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.me(None).await.unwrap().is_none());
    assert!(log.last().starts_with("GET /api/me "), "{}", log.last());
}

#[tokio::test]
async fn a_refusal_says_why() {
    let (base, _) = board(answer(
        "401 Unauthorized",
        &["content-type: application/json"],
        "{\"error\":\"invalid credentials\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.sign_in("alice", "wrong").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 401);
            assert_eq!(reason, "invalid credentials");
        }
        other => panic!("a refusal must carry its reason: {other:?}"),
    }
}

#[tokio::test]
async fn a_refusal_without_words_is_kept_as_a_status() {
    let (base, _) = board(answer(
        "409 Conflict",
        &["content-type: application/json"],
        "{}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client
            .register(&client::RegisterBody::new(
                "alice",
                "a@example.org",
                "secret11"
            ))
            .await
            .unwrap_err(),
        ClientError::Status(409)
    );
}

#[tokio::test]
async fn an_unreadable_account_is_an_error() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        "not json",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.me(Some("tok9")).await {
        Err(ClientError::Detail(_)) => {}
        other => panic!("an unreadable answer must be an error: {other:?}"),
    }
}
