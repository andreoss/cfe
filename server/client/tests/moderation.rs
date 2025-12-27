use client::{ApiClient, ClientError};
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

fn answer(status: &'static str, body: &'static str) -> &'static str {
    let mut response = format!("HTTP/1.1 {status}\r\n");
    response.push_str("content-type: application/json\r\n");
    response.push_str(&format!("content-length: {}\r\n", body.len()));
    response.push_str("connection: close\r\n\r\n");
    Box::leak(Box::new(response + body))
}

const WARNINGS: &str = concat!(
    "[{\"id\":\"77777777-7777-7777-7777-777777777777\",",
    "\"reason\":\"spam\",\"created_at\":\"2024-12-17T23:00:00Z\",\"acknowledged\":false},",
    "{\"id\":\"88888888-8888-8888-8888-888888888888\",",
    "\"reason\":\"flood\",\"created_at\":\"2024-12-17T23:10:00Z\",\"acknowledged\":true}]"
);

const IGNORED: &str = "{\"ignored\":true}";
const NOT_IGNORED: &str = "{\"ignored\":false}";
const BANNED: &str = "{\"banned\":true,\"reason\":\"spam\",\"until\":null}";
const NOT_BANNED: &str = "{\"banned\":false,\"reason\":null,\"until\":null}";

const PUT_BAN: &str = "{\"reason\":\"spam\",\"until\":\"2024-12-25T00:00:00Z\"}";
const WARNING: &str = concat!(
    "{\"id\":\"99999999-9999-9999-9999-999999999999\",",
    "\"reason\":\"spam\",\"created_at\":\"2024-12-18T00:00:00Z\",\"acknowledged\":false}"
);
const ACCOUNT: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"username\":\"bob_02\",\"role\":\"moderator\"}"
);
const REMARK: &str = "{\"text\":\"keeps the board clean\"}";
const NO_REMARK: &str = "{\"text\":null}";

const NAME: &str = "bob_02";

#[tokio::test]
async fn the_warnings_of_an_account_are_read_with_the_session() {
    let (base, log) = board(answer("200 OK", WARNINGS)).await;
    let client = ApiClient::new(&base).unwrap();
    let warnings = client.warnings("token").await.unwrap();
    assert_eq!(warnings.len(), 2);
    assert_eq!(
        warnings[0].id,
        "77777777-7777-7777-7777-777777777777".to_owned()
    );
    assert_eq!(warnings[0].reason, "spam".to_owned());
    assert_eq!(warnings[0].created_at, "2024-12-17T23:00:00Z".to_owned());
    assert!(!warnings[0].acknowledged);
    assert!(warnings[1].acknowledged);
    let request = log.last();
    assert!(request.starts_with("GET /api/me/warnings "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_board_without_warnings_gives_an_empty_list() {
    let (base, _) = board(answer("200 OK", "[]")).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.warnings("token").await.unwrap().is_empty());
}

#[tokio::test]
async fn the_warnings_are_acknowledged_with_the_session() {
    let (base, log) = board(answer("204 No Content", "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.acknowledge_warnings("token").await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/me/warnings/acknowledge "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn acknowledging_is_refused_without_a_session() {
    let (base, _) = board(answer("401 Unauthorized", "{\"error\":\"sign in\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    let error = client.acknowledge_warnings("token").await.unwrap_err();
    assert_eq!(error.status(), Some(401));
}

#[tokio::test]
async fn whether_an_account_is_ignored_is_read_with_the_session() {
    let (base, log) = board(answer("200 OK", IGNORED)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.ignore_state("token", NAME).await.unwrap());
    let request = log.last();
    assert!(
        request.starts_with("GET /api/users/bob_02/ignore "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_account_that_is_not_ignored_says_so() {
    let (base, _) = board(answer("200 OK", NOT_IGNORED)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(!client.ignore_state("token", NAME).await.unwrap());
}

#[tokio::test]
async fn an_odd_name_stays_inside_one_step_of_the_ignore_address() {
    let (base, log) = board(answer("200 OK", NOT_IGNORED)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(!client.ignore_state("token", "bob 02/x").await.unwrap());
    assert!(
        log.last()
            .starts_with("GET /api/users/bob%2002%2Fx/ignore "),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn the_ban_state_of_an_account_is_read_with_the_session() {
    let (base, log) = board(answer("200 OK", BANNED)).await;
    let client = ApiClient::new(&base).unwrap();
    let ban = client.ban_state("token", NAME).await.unwrap().unwrap();
    assert!(ban.banned);
    assert_eq!(ban.reason, Some("spam".to_owned()));
    assert_eq!(ban.until, None);
    let request = log.last();
    assert!(
        request.starts_with("GET /api/users/bob_02/ban "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_account_that_is_not_banned_has_no_ban() {
    let (base, _) = board(answer("200 OK", NOT_BANNED)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.ban_state("token", NAME).await.unwrap().is_none());
}

#[tokio::test]
async fn an_account_that_is_not_there_has_no_ban() {
    let (base, _) = board(answer("404 Not Found", "{\"error\":\"user not found\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.ban_state("token", NAME).await.unwrap().is_none());
}

#[tokio::test]
async fn a_ban_state_an_account_may_not_read_is_refused() {
    let (base, _) = board(answer(
        "403 Forbidden",
        "{\"error\":\"moderator role required\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let error = client.ban_state("token", NAME).await.unwrap_err();
    assert_eq!(error.status(), Some(403));
    assert!(
        matches!(error, ClientError::Rejected { reason, .. } if reason == "moderator role required")
    );
}

#[tokio::test]
async fn an_account_is_banned_with_a_reason_and_a_number_of_days() {
    let (base, log) = board(answer("200 OK", PUT_BAN)).await;
    let client = ApiClient::new(&base).unwrap();
    let ban = client.ban("token", NAME, "spam", Some(3)).await.unwrap();
    assert_eq!(ban.reason, "spam".to_owned());
    assert_eq!(ban.until, Some("2024-12-25T00:00:00Z".to_owned()));
    let request = log.last();
    assert!(
        request.starts_with("POST /api/users/bob_02/ban "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(
        request.ends_with("{\"reason\":\"spam\",\"days\":3}"),
        "{request}"
    );
}

#[tokio::test]
async fn an_account_is_banned_without_an_end() {
    let (base, log) = board(answer("200 OK", PUT_BAN)).await;
    let client = ApiClient::new(&base).unwrap();
    client.ban("token", NAME, "spam", None).await.unwrap();
    assert!(
        log.last().ends_with("{\"reason\":\"spam\",\"days\":null}"),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn an_odd_name_stays_inside_one_step_of_the_ban_address() {
    let (base, log) = board(answer("200 OK", PUT_BAN)).await;
    let client = ApiClient::new(&base).unwrap();
    client.ban("token", "bob 02/x", "spam", None).await.unwrap();
    assert!(
        log.last().starts_with("POST /api/users/bob%2002%2Fx/ban "),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn a_ban_an_account_may_not_put_on_another_is_refused() {
    let (base, _) = board(answer(
        "403 Forbidden",
        "{\"error\":\"moderator role required\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let error = client.ban("token", NAME, "spam", None).await.unwrap_err();
    assert_eq!(error.status(), Some(403));
}

#[tokio::test]
async fn the_ban_of_an_account_is_lifted_with_the_session() {
    let (base, log) = board(answer("204 No Content", "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.lift_ban("token", NAME).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("DELETE /api/users/bob_02/ban "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_account_is_warned_with_a_reason() {
    let (base, log) = board(answer("200 OK", WARNING)).await;
    let client = ApiClient::new(&base).unwrap();
    let warning = client.warn("token", NAME, "spam").await.unwrap();
    assert_eq!(warning.reason, "spam".to_owned());
    assert!(!warning.acknowledged);
    let request = log.last();
    assert!(
        request.starts_with("POST /api/users/bob_02/warn "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.ends_with("{\"reason\":\"spam\"}"), "{request}");
}

#[tokio::test]
async fn an_account_is_promoted_with_the_session() {
    let (base, log) = board(answer("200 OK", ACCOUNT)).await;
    let client = ApiClient::new(&base).unwrap();
    let account = client.promote("token", NAME).await.unwrap();
    assert_eq!(account.username, "bob_02".to_owned());
    assert_eq!(account.role, "moderator".to_owned());
    let request = log.last();
    assert!(
        request.starts_with("POST /api/users/bob_02/promote "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_role_of_an_account_is_set_with_the_session() {
    let (base, log) = board(answer("200 OK", ACCOUNT)).await;
    let client = ApiClient::new(&base).unwrap();
    let account = client.set_role("token", NAME, "corrector").await.unwrap();
    assert_eq!(account.role, "moderator".to_owned());
    let request = log.last();
    assert!(
        request.starts_with("POST /api/users/bob_02/role "),
        "{request}"
    );
    assert!(request.ends_with("{\"role\":\"corrector\"}"), "{request}");
}

#[tokio::test]
async fn an_account_ignores_another_with_the_session() {
    let (base, log) = board(answer("200 OK", IGNORED)).await;
    let client = ApiClient::new(&base).unwrap();
    client.ignore("token", NAME).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/users/bob_02/ignore "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_account_stops_ignoring_another_with_the_session() {
    let (base, log) = board(answer("200 OK", NOT_IGNORED)).await;
    let client = ApiClient::new(&base).unwrap();
    client.stop_ignoring("token", NAME).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("DELETE /api/users/bob_02/ignore "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_remark_about_an_account_is_read_with_the_session() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    let remark = client.remark("token", NAME).await.unwrap().unwrap();
    assert_eq!(remark, "keeps the board clean".to_owned());
    let request = log.last();
    assert!(
        request.starts_with("GET /api/users/bob_02/remark "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_account_with_no_remark_about_it_has_none() {
    let (base, _) = board(answer("200 OK", NO_REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.remark("token", NAME).await.unwrap().is_none());
}

#[tokio::test]
async fn a_remark_about_an_account_is_kept_with_the_session() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    let remark = client
        .set_remark("token", NAME, "keeps the board clean")
        .await
        .unwrap();
    assert_eq!(remark, "keeps the board clean".to_owned());
    let request = log.last();
    assert!(
        request.starts_with("PUT /api/users/bob_02/remark "),
        "{request}"
    );
    assert!(
        request.ends_with("{\"text\":\"keeps the board clean\"}"),
        "{request}"
    );
}

#[tokio::test]
async fn a_remark_about_oneself_is_refused() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        "{\"error\":\"not about yourself\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let error = client.set_remark("token", NAME, "mine").await.unwrap_err();
    assert_eq!(error.status(), Some(422));
}

#[tokio::test]
async fn the_remark_about_an_account_is_taken_away_with_the_session() {
    let (base, log) = board(answer("204 No Content", "")).await;
    let client = ApiClient::new(&base).unwrap();
    client.clear_remark("token", NAME).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("DELETE /api/users/bob_02/remark "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}
