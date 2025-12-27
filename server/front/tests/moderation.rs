use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECTIONS: &str =
    "[{\"slug\":\"general\",\"title\":\"General\",\"topics_score\":\"0\",\"may_post\":true}]";

const PROFILE: &str = concat!(
    "{\"id\":\"9\",\"username\":\"bob\",\"bio\":null,",
    "\"score\":10,\"role\":\"user\"}"
);

const WARNINGS: &str = concat!(
    "[{\"id\":\"77777777-7777-7777-7777-777777777777\",",
    "\"reason\":\"spam\",\"created_at\":\"2024-12-17T23:00:00Z\",\"acknowledged\":false},",
    "{\"id\":\"88888888-8888-8888-8888-888888888888\",",
    "\"reason\":\"flood\",\"created_at\":\"2024-12-17T23:10:00Z\",\"acknowledged\":true}]"
);

const IGNORED: &str = "{\"ignored\":true}";
const NOT_IGNORED: &str = "{\"ignored\":false}";
const BANNED: &str = "{\"banned\":true,\"reason\":\"spam\",\"until\":\"2024-12-19T23:00:00Z\"}";
const NOT_BANNED: &str = "{\"banned\":false,\"reason\":null,\"until\":null}";
const GIVEN_BAN: &str = "{\"reason\":\"spam\",\"until\":null}";
const GIVEN_WARNING: &str = concat!(
    "{\"id\":\"99999999-9999-9999-9999-999999999999\",",
    "\"reason\":\"spam\",\"created_at\":\"2024-12-18T00:00:00Z\",\"acknowledged\":false}"
);
const PROMOTED: &str = "{\"id\":\"9\",\"username\":\"bob\",\"role\":\"moderator\"}";
const GIVEN_REPORT: &str = concat!(
    "{\"id\":\"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"comment_id\":null,\"reporter_username\":\"bob\",",
    "\"kind\":\"rule\",\"reason\":\"spam\",",
    "\"created_at\":\"2024-12-18T01:00:00Z\"}"
);
const REPORTS: &str = concat!(
    "{\"items\":[{\"id\":\"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"comment_id\":\"22222222-2222-2222-2222-222222222222\",",
    "\"reporter_username\":\"bob\",\"kind\":\"spelling\",",
    "\"reason\":\"typos\",\"created_at\":\"2024-12-18T01:00:00Z\"}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);
const REMARK: &str = "{\"text\":\"keeps the board clean\"}";
const EMPTY: &str = "";
const REFUSED: &str = "{\"error\":\"moderator role required\"}";

fn user(role: &str) -> String {
    format!("{{\"id\":\"9\",\"username\":\"alice\",\"role\":\"{role}\"}}")
}

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(role: &str, warnings: &str, ignored: bool, banned: bool) -> (String, Log) {
    let role = role.to_owned();
    let warnings = warnings.to_owned();
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
            let role = role.clone();
            let warnings = warnings.clone();
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 16384];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let mut parts = request.split_whitespace();
                let method = parts.next().unwrap_or("GET").to_owned();
                let target = parts.next().unwrap_or("/").to_owned();
                sink.lock().unwrap().push(format!("{method} {target}"));
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (status, body) = match path.as_str() {
                    "/api/me" => ("200 OK", user(&role)),
                    "/api/sections" => ("200 OK", SECTIONS.to_owned()),
                    "/api/me/warnings" => ("200 OK", warnings),
                    "/api/me/warnings/acknowledge" => ("204 No Content", String::new()),
                    "/api/users/bob" => ("200 OK", PROFILE.to_owned()),
                    "/api/users/bob/ignore" => (
                        "200 OK",
                        if ignored {
                            IGNORED.to_owned()
                        } else {
                            NOT_IGNORED.to_owned()
                        },
                    ),
                    "/api/users/bob/ban" => {
                        if role == "moderator" {
                            match method.as_str() {
                                "POST" => ("200 OK", GIVEN_BAN.to_owned()),
                                "DELETE" => ("204 No Content", EMPTY.to_owned()),
                                _ => (
                                    "200 OK",
                                    if banned {
                                        BANNED.to_owned()
                                    } else {
                                        NOT_BANNED.to_owned()
                                    },
                                ),
                            }
                        } else {
                            ("403 Forbidden", REFUSED.to_owned())
                        }
                    }
                    "/api/users/bob/warn" => ("200 OK", GIVEN_WARNING.to_owned()),
                    "/api/users/bob/promote" => ("200 OK", PROMOTED.to_owned()),
                    "/api/users/bob/role" => ("200 OK", PROMOTED.to_owned()),
                    "/api/users/bob/remark" => match method.as_str() {
                        "DELETE" => ("204 No Content", EMPTY.to_owned()),
                        _ => ("200 OK", REMARK.to_owned()),
                    },
                    "/api/reports" => ("200 OK", REPORTS.to_owned()),
                    path if path.starts_with("/api/reports/") && path.ends_with("/close") => {
                        ("200 OK", GIVEN_REPORT.to_owned())
                    }
                    path if path.starts_with("/api/topics/") && path.ends_with("/report") => {
                        ("200 OK", GIVEN_REPORT.to_owned())
                    }
                    _ => ("404 Not Found", "{\"error\":\"no\"}".to_owned()),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    (format!("http://{address}"), log)
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

async fn front(server_url: &str) -> String {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(config::SERVER_URL.to_owned(), server_url.to_owned());
    let config = Config::from_vars(&vars);
    let app = front::App::new(&config).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, routes::router(app)).await;
    });
    format!("http://{address}")
}

async fn get(base: &str, address: &str, session: Option<&str>) -> reqwest::Response {
    let request = http().get(format!("{base}{address}"));
    let request = match session {
        Some(token) => request.header("cookie", format!("session={token}")),
        None => request,
    };
    request.send().await.unwrap()
}

async fn token_of(base: &str, address: &str) -> String {
    let response = get(base, address, Some("token")).await;
    let token = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(';').next())
        .and_then(|pair| pair.strip_prefix("token="))
        .expect("a page answers with a token")
        .to_owned();
    response.text().await.unwrap();
    token
}

async fn post_form(
    base: &str,
    action: &str,
    fields: &[(&str, &str)],
    token: Option<&str>,
) -> reqwest::Response {
    let cookie = match token {
        Some(token) => format!("session=token; token={token}"),
        None => "session=token".to_owned(),
    };
    http()
        .post(format!("{base}{action}"))
        .header("cookie", cookie)
        .form(&fields.to_vec())
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn the_warnings_of_an_account_are_on_a_page_of_their_own() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/me/warnings", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("spam"), "{page}");
    assert!(page.contains("flood"), "{page}");
    assert!(page.contains("2024-12-17T23:00:00Z"), "{page}");
    assert!(page.contains("open"), "{page}");
    assert!(page.contains("acknowledged"), "{page}");
    assert!(
        page.contains("action=\"/me/warnings/acknowledge\""),
        "{page}"
    );
    assert!(!page.contains("<script"), "{page}");
    assert!(log.calls().contains(&"GET /api/me/warnings".to_owned()));
}

#[tokio::test]
async fn an_account_with_no_warnings_is_told_so() {
    let (board_url, _) = board("user", "[]", false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/me/warnings", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("No warnings"), "{page}");
    assert!(!page.contains("me/warnings/acknowledge"), "{page}");
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in() {
    let (board_url, _) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let response = get(&base, "/me/warnings", None).await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/sign-in?return_to=/me/warnings")
    );
}

#[tokio::test]
async fn acknowledging_the_warnings_goes_back_to_the_page() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/me/warnings").await;
    let response = post_form(
        &base,
        "/me/warnings/acknowledge",
        &[("token", &token)],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/me/warnings")
    );
    assert!(
        log.calls()
            .contains(&"POST /api/me/warnings/acknowledge".to_owned())
    );
}

#[tokio::test]
async fn acknowledging_without_the_token_is_refused_and_leaves_the_board_alone() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let response = post_form(
        &base,
        "/me/warnings/acknowledge",
        &[("token", "not-mine")],
        None,
    )
    .await;
    assert_eq!(response.status(), 403);
    assert!(
        !log.calls().iter().any(|call| call.contains("acknowledge")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_page_of_an_account_says_whether_one_ignores_it() {
    let (board_url, log) = board("user", WARNINGS, true, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("You ignore this account"), "{page}");
    assert!(
        log.calls()
            .contains(&"GET /api/users/bob/ignore".to_owned())
    );
}

#[tokio::test]
async fn an_account_one_does_not_ignore_says_so() {
    let (board_url, _) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("You do not ignore this account"), "{page}");
}

#[tokio::test]
async fn a_moderator_is_told_an_account_is_banned() {
    let (board_url, log) = board("moderator", WARNINGS, false, true).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("This account is banned"), "{page}");
    assert!(page.contains("spam"), "{page}");
    assert!(page.contains("2024-12-19T23:00:00Z"), "{page}");
    assert!(log.calls().contains(&"GET /api/users/bob/ban".to_owned()));
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn a_moderator_is_told_an_account_is_not_banned() {
    let (board_url, _) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("This account is not banned"), "{page}");
}

#[tokio::test]
async fn an_account_that_does_not_keep_the_board_is_not_told_about_bans() {
    let (board_url, log) = board("user", WARNINGS, false, true).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(!page.contains("banned"), "{page}");
    assert!(
        !log.calls().iter().any(|call| call.contains("/ban")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_page_of_an_account_offers_a_moderator_its_ways() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/u/bob/ban\""), "{page}");
    assert!(page.contains("action=\"/u/bob/warn\""), "{page}");
    assert!(page.contains("action=\"/u/bob/promote\""), "{page}");
    assert!(page.contains("action=\"/u/bob/role\""), "{page}");
    assert!(page.contains("action=\"/u/bob/remark\""), "{page}");
    assert!(page.contains("keeps the board clean"), "{page}");
    assert!(page.contains("action=\"/u/bob/remark/remove\""), "{page}");
    assert!(!page.contains("<script"), "{page}");
    assert!(
        log.calls()
            .contains(&"GET /api/users/bob/remark".to_owned())
    );
}

#[tokio::test]
async fn an_account_that_does_not_keep_the_board_is_offered_none_of_them() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(!page.contains("action=\"/u/bob/ban\""), "{page}");
    assert!(!page.contains("action=\"/u/bob/warn\""), "{page}");
    assert!(!page.contains("action=\"/u/bob/promote\""), "{page}");
    assert!(!page.contains("action=\"/u/bob/role\""), "{page}");
    assert!(!page.contains("action=\"/u/bob/remark\""), "{page}");
    assert!(
        !log.calls().iter().any(|call| call.contains("remark")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_way_to_ban_an_account_carries_the_reason_and_the_days() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/u/bob").await;
    let response = post_form(
        &base,
        "/u/bob/ban",
        &[("token", token.as_str()), ("reason", "spam"), ("days", "3")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/u/bob")
    );
    assert!(log.calls().contains(&"POST /api/users/bob/ban".to_owned()));
}

#[tokio::test]
async fn the_ban_of_an_account_is_lifted_from_its_page() {
    let (board_url, log) = board("moderator", WARNINGS, false, true).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/u/bob").await;
    let response = post_form(&base, "/u/bob/ban/lift", &[("token", &token)], Some(&token)).await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls()
            .contains(&"DELETE /api/users/bob/ban".to_owned())
    );
}

#[tokio::test]
async fn the_way_to_warn_an_account_carries_the_reason() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/u/bob").await;
    let response = post_form(
        &base,
        "/u/bob/warn",
        &[("token", token.as_str()), ("reason", "spam")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert!(log.calls().contains(&"POST /api/users/bob/warn".to_owned()));
}

#[tokio::test]
async fn the_role_of_an_account_is_set_from_its_page() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/u/bob").await;
    let promote = post_form(&base, "/u/bob/promote", &[("token", &token)], Some(&token)).await;
    assert_eq!(promote.status(), 303);
    assert!(
        log.calls()
            .contains(&"POST /api/users/bob/promote".to_owned())
    );
    let role = post_form(
        &base,
        "/u/bob/role",
        &[("token", token.as_str()), ("role", "corrector")],
        Some(&token),
    )
    .await;
    assert_eq!(role.status(), 303);
    assert!(log.calls().contains(&"POST /api/users/bob/role".to_owned()));
}

#[tokio::test]
async fn a_way_of_moderation_without_the_token_of_the_page_is_refused() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let response = post_form(
        &base,
        "/u/bob/ban",
        &[("token", "not-mine"), ("reason", "spam")],
        None,
    )
    .await;
    assert_eq!(response.status(), 403);
    assert!(
        !log.calls()
            .iter()
            .any(|call| call.starts_with("POST /api/users/bob/ban")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_way_to_ignore_an_account_is_on_its_page() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/u/bob").await;
    let response = post_form(&base, "/u/bob/ignore", &[("token", &token)], Some(&token)).await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls()
            .contains(&"POST /api/users/bob/ignore".to_owned())
    );
}

#[tokio::test]
async fn an_account_one_ignores_is_offered_the_way_to_stop() {
    let (board_url, _) = board("user", WARNINGS, true, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/u/bob", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("action=\"/u/bob/ignore/stop\""), "{page}");
    assert!(!page.contains("action=\"/u/bob/ignore\""), "{page}");
}

#[tokio::test]
async fn a_remark_about_an_account_is_kept_and_taken_away() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/u/bob").await;
    let kept = post_form(
        &base,
        "/u/bob/remark",
        &[("token", token.as_str()), ("text", "keeps the board clean")],
        Some(&token),
    )
    .await;
    assert_eq!(kept.status(), 303);
    assert!(
        log.calls()
            .contains(&"PUT /api/users/bob/remark".to_owned())
    );
    let gone = post_form(
        &base,
        "/u/bob/remark/remove",
        &[("token", &token)],
        Some(&token),
    )
    .await;
    assert_eq!(gone.status(), 303);
    assert!(
        log.calls()
            .contains(&"DELETE /api/users/bob/remark".to_owned())
    );
}

const SUBJECT: &str = "11111111-1111-1111-1111-111111111111";
const REMARK_ID: &str = "22222222-2222-2222-2222-222222222222";

#[tokio::test]
async fn the_reports_of_the_board_are_on_a_page_of_their_own() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = get(&base, "/reports", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("spelling"), "{page}");
    assert!(page.contains("typos"), "{page}");
    assert!(page.contains(">bob<"), "{page}");
    assert!(
        page.contains("action=\"/reports/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/close\""),
        "{page}"
    );
    assert!(!page.contains("<script"), "{page}");
    assert!(log.calls().contains(&"GET /api/reports?page=1".to_owned()));
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in_before_the_reports() {
    let (board_url, _) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let response = get(&base, "/reports", None).await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/sign-in?return_to=/reports")
    );
}

#[tokio::test]
async fn closing_a_report_goes_back_to_the_reports() {
    let (board_url, log) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/reports").await;
    let response = post_form(
        &base,
        "/reports/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/close",
        &[("token", &token)],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/reports")
    );
    assert!(
        log.calls()
            .contains(&"POST /api/reports/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/close".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_report_closed_without_the_token_of_the_page_is_refused() {
    let (board_url, _) = board("moderator", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let response = post_form(
        &base,
        "/reports/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/close",
        &[("token", "other")],
        None,
    )
    .await;
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn a_subject_is_reported_from_its_page() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let page = format!("/topics/{SUBJECT}");
    let token = token_of(&base, "/reports").await;
    let response = post_form(
        &base,
        &format!("{page}/report"),
        &[("token", &token), ("kind", "rule"), ("reason", "spam")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some(page.as_str())
    );
    assert!(
        log.calls()
            .contains(&format!("POST /api/topics/{SUBJECT}/report")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_remark_of_a_subject_is_reported_from_its_page() {
    let (board_url, log) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/reports").await;
    let response = post_form(
        &base,
        &format!("/topics/{SUBJECT}/comments/{REMARK_ID}/report"),
        &[("token", &token), ("kind", "spelling"), ("reason", "typos")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls().contains(&format!(
            "POST /api/topics/{SUBJECT}/comments/{REMARK_ID}/report"
        )),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_report_without_the_token_of_the_page_is_refused() {
    let (board_url, _) = board("user", WARNINGS, false, false).await;
    let base = front(&board_url).await;
    let response = post_form(
        &base,
        &format!("/topics/{SUBJECT}/report"),
        &[("token", "other"), ("kind", "rule"), ("reason", "spam")],
        None,
    )
    .await;
    assert_eq!(response.status(), 403);
}
