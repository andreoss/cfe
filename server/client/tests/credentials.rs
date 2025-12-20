use client::{ApiClient, ClientError, User};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn last(&self) -> String {
        self.0.lock().unwrap().last().cloned().unwrap_or_default()
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
async fn a_new_password_is_carried_with_the_old_one_and_the_session() {
    let (base, log) = board(answer(
        "200 OK",
        &[
            "content-type: application/json",
            "set-cookie: session=new9; Path=/; HttpOnly",
        ],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let cookie = client
        .change_password("tok9", "secret11", "secret22")
        .await
        .unwrap();
    assert_eq!(cookie.as_deref(), Some("session=new9; Path=/; HttpOnly"));
    let request = log.last();
    assert!(request.starts_with("POST /api/me/password "), "{request}");
    assert!(request.contains("cookie: session=tok9"), "{request}");
    assert!(
        request.contains("\"current_password\":\"secret11\""),
        "{request}"
    );
    assert!(
        request.contains("\"new_password\":\"secret22\""),
        "{request}"
    );
}

#[tokio::test]
async fn a_password_the_board_refuses_keeps_its_reason() {
    let (base, _) = board(answer(
        "401 Unauthorized",
        &["content-type: application/json"],
        "{\"error\":\"wrong current password\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.change_password("tok9", "secret11", "secret22").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 401);
            assert_eq!(reason, "wrong current password");
        }
        other => panic!("a refused password must keep its reason: {other:?}"),
    }
}

#[tokio::test]
async fn a_recovery_is_asked_for_by_the_address_on_file() {
    let (base, log) = board(answer("202 Accepted", &[], "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.request_reset("a@example.org").await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/password-reset "),
        "{request}"
    );
    assert!(request.contains("\"email\":\"a@example.org\""), "{request}");
}

#[tokio::test]
async fn an_address_the_board_does_not_hold_is_still_accepted() {
    let (base, _) = board(answer("202 Accepted", &[], "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.request_reset("nobody@example.org").await.unwrap();
}

#[tokio::test]
async fn a_recovery_is_confirmed_with_the_secret_and_the_new_password() {
    let (base, log) = board(answer("204 No Content", &[], "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.reset_password("code-1", "secret22").await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/password-reset/confirm "),
        "{request}"
    );
    assert!(request.contains("\"code\":\"code-1\""), "{request}");
    assert!(
        request.contains("\"new_password\":\"secret22\""),
        "{request}"
    );
}

#[tokio::test]
async fn a_secret_the_board_does_not_know_is_refused() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        &["content-type: application/json"],
        "{\"error\":\"invalid or expired code\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.reset_password("code-1", "secret22").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 422);
            assert_eq!(reason, "invalid or expired code");
        }
        other => panic!("a secret nobody knows must be refused: {other:?}"),
    }
}

#[tokio::test]
async fn a_new_address_is_asked_for_with_the_session() {
    let (base, log) = board(answer("202 Accepted", &[], "")).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .request_email_change("tok9", "b@example.org")
        .await
        .unwrap();
    let request = log.last();
    assert!(request.starts_with("POST /api/me/email "), "{request}");
    assert!(request.contains("cookie: session=tok9"), "{request}");
    assert!(request.contains("\"email\":\"b@example.org\""), "{request}");
}

#[tokio::test]
async fn an_address_somebody_else_holds_is_refused() {
    let (base, _) = board(answer(
        "409 Conflict",
        &["content-type: application/json"],
        "{\"error\":\"address taken\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.request_email_change("tok9", "b@example.org").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 409);
            assert_eq!(reason, "address taken");
        }
        other => panic!("an address somebody holds must be refused: {other:?}"),
    }
}

#[tokio::test]
async fn an_address_is_confirmed_with_the_secret() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let user = client.confirm_email("code-1").await.unwrap();
    assert_eq!(
        user,
        User {
            id: "7f2c".to_owned(),
            username: "alice".to_owned(),
            role: "user".to_owned(),
        }
    );
    let request = log.last();
    assert!(
        request.starts_with("POST /api/me/email/confirm "),
        "{request}"
    );
    assert!(request.contains("\"code\":\"code-1\""), "{request}");
}

#[tokio::test]
async fn an_account_is_activated_with_the_secret() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(client.activate("code-1").await.unwrap().username, "alice");
    let request = log.last();
    assert!(request.starts_with("POST /api/activate "), "{request}");
    assert!(request.contains("\"code\":\"code-1\""), "{request}");
}

#[tokio::test]
async fn a_deregistration_brings_back_the_cookie_that_drops_the_session() {
    let (base, log) = board(answer(
        "200 OK",
        &[
            "content-type: application/json",
            "set-cookie: session=; Path=/; Max-Age=0",
        ],
        ACCOUNT,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.deregister("tok9").await.unwrap(),
        Some("session=; Path=/; Max-Age=0".to_owned())
    );
    let request = log.last();
    assert!(request.starts_with("POST /api/me/deregister "), "{request}");
    assert!(request.contains("cookie: session=tok9"), "{request}");
}
