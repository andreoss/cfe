use front::config;
use front::config::Config;
use front::routes;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SUBJECT: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"group_slug\":null,",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"tags\":[\"rust\"],\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"postscore\":0,",
    "\"pending\":false,\"draft\":false,\"sticky\":false,",
    "\"off_front\":false,\"resolved\":false,\"minor\":false,",
    "\"open_reports\":0}"
);

const REMOVED: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"group_slug\":null,",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"tags\":[\"rust\"],\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":true,",
    "\"deleted_reason\":\"off topic\",\"edited\":false,\"postscore\":0,",
    "\"pending\":false,\"draft\":false,\"sticky\":false,",
    "\"off_front\":false,\"resolved\":false,\"minor\":false,",
    "\"open_reports\":0}"
);

const REMARK: &str = concat!(
    "{\"id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"parent_id\":null,\"body\":\"a remark on it\",",
    "\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T11:12:13Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"ignored\":false}"
);

const REMARKS: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"parent_id\":null,\"body\":\"a remark on it\",",
    "\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T11:12:13Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"ignored\":false}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const VERSIONS: &str = concat!(
    "[{\"id\":\"44444444-4444-4444-4444-444444444444\",",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"editor\":\"alice\",\"written_at\":\"2024-06-07T10:11:12Z\"},",
    "{\"id\":\"55555555-5555-5555-5555-555555555555\",",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it, mended\",",
    "\"editor\":\"bob\",\"written_at\":\"2024-06-07T12:13:14Z\"}]"
);

const NO_VERSIONS: &str = "[]";

const CHANGES: &str = concat!(
    "[{\"kind\":\"added\",\"line\":\"the body of it, mended\"},",
    "{\"kind\":\"removed\",\"line\":\"the body of it\"}]"
);

const USER: &str = "{\"id\":\"9\",\"username\":\"alice\",\"role\":\"user\"}";
const REFUSED: &str = "{\"error\":\"not allowed to change this\"}";

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const REMARK_ID: &str = "33333333-3333-3333-3333-333333333333";
const VERSION_ID: &str = "44444444-4444-4444-4444-444444444444";

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(
    subject: &'static str,
    written: &'static str,
    reason: &'static str,
    versions: &'static str,
) -> (String, Log) {
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
                let mut parts = request.split_whitespace();
                let method = parts.next().unwrap_or("GET").to_owned();
                let target = parts.next().unwrap_or("/").to_owned();
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
                sink.lock().unwrap().push(format!("{method} {target}"));
                if !body.is_empty() {
                    sink.lock().unwrap().push(format!("BODY {body}"));
                }
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (answer, body) = match path.as_str() {
                    "/api/me" => ("200 OK", USER),
                    "/api/topics/11111111-1111-1111-1111-111111111111" => match method.as_str() {
                        "PATCH" => (reason, written),
                        _ => ("200 OK", subject),
                    },
                    "/api/topics/11111111-1111-1111-1111-111111111111/comments" => {
                        ("200 OK", REMARKS)
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/delete" => (reason, written),
                    "/api/topics/11111111-1111-1111-1111-111111111111/restore" => (reason, written),
                    "/api/topics/11111111-1111-1111-1111-111111111111/history" => {
                        ("200 OK", versions)
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/history/44444444-4444-4444-4444-444444444444" => {
                        ("200 OK", CHANGES)
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/comments/33333333-3333-3333-3333-333333333333" => {
                        (reason, written)
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/comments/33333333-3333-3333-3333-333333333333/delete" => {
                        (reason, written)
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/comments/33333333-3333-3333-3333-333333333333/restore" => {
                        (reason, written)
                    }
                    _ => ("404 Not Found", "{\"error\":\"not found\"}"),
                };
                let response = format!(
                    "HTTP/1.1 {answer}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
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
    let mut vars = BTreeMap::new();
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

async fn form(base: &str, page: &str, action: &str, fields: &[(&str, &str)]) -> reqwest::Response {
    let token = token_of(base, page).await;
    let mut pairs: Vec<(&str, &str)> = vec![("token", token.as_str())];
    pairs.extend_from_slice(fields);
    http()
        .post(format!("{base}{action}"))
        .header("cookie", format!("session=token; token={token}"))
        .form(&pairs)
        .send()
        .await
        .unwrap()
}

async fn sent(base: &str, action: &str, fields: &[(&str, &str)]) -> reqwest::Response {
    http()
        .post(format!("{base}{action}"))
        .header("cookie", "session=token")
        .form(&fields.to_vec())
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn the_page_to_change_a_subject_carries_what_was_written() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}/edit"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Change a subject</h2>"), "{page}");
    assert!(
        page.contains(&format!(
            "<form class=\"subject-form\" method=\"post\" action=\"/topics/{SUBJECT_ID}/edit\">"
        )),
        "{page}"
    );
    assert!(page.contains("value=\"Ports and adapters\""), "{page}");
    assert!(page.contains(">the body of it</textarea>"), "{page}");
    assert!(page.contains("value=\"rust\""), "{page}");
    assert!(page.contains("name=\"token\""), "{page}");
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in_before_changing() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = get(&base, &format!("/topics/{SUBJECT_ID}/edit"), None).await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/sign-in?return_to=/topics/{SUBJECT_ID}/edit")
    );
}

#[tokio::test]
async fn a_change_written_from_the_page_goes_to_the_board_with_the_session() {
    let (board_url, log) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/edit"),
        &format!("/topics/{SUBJECT_ID}/edit"),
        &[
            ("title", "Ports and adapters, mended"),
            ("body", "the body of it, mended"),
            ("tags", "rust, ports"),
        ],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}")
    );
    let calls = log.calls();
    assert!(
        calls.contains(&format!("PATCH /api/topics/{SUBJECT_ID}")),
        "{calls:?}"
    );
    let body = calls
        .iter()
        .find(|call| call.starts_with("BODY {"))
        .expect("the board is written to with a body");
    assert!(
        body.contains("\"title\":\"Ports and adapters, mended\""),
        "{body}"
    );
    assert!(body.contains("\"tags\":[\"rust\",\"ports\"]"), "{body}");
}

#[tokio::test]
async fn a_change_written_without_the_token_of_its_page_is_refused() {
    let (board_url, log) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = sent(
        &base,
        &format!("/topics/{SUBJECT_ID}/edit"),
        &[("title", "Ports"), ("body", "the body")],
    )
    .await;
    assert_eq!(answer.status(), 403);
    assert!(
        !log.calls()
            .contains(&format!("PATCH /api/topics/{SUBJECT_ID}")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_board_that_refuses_a_change_keeps_its_reason() {
    let (board_url, _) = board(SUBJECT, REFUSED, "403 Forbidden", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/edit"),
        &format!("/topics/{SUBJECT_ID}/edit"),
        &[("title", "Ports"), ("body", "the body")],
    )
    .await;
    assert_eq!(answer.status(), 422);
    let page = answer.text().await.unwrap();
    assert!(page.contains("not allowed to change this"), "{page}");
}

#[tokio::test]
async fn the_page_to_change_a_remark_carries_what_was_said() {
    let (board_url, _) = board(SUBJECT, REMARK, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/edit"),
        Some("token"),
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(page.contains("<h2>Change a remark</h2>"), "{page}");
    assert!(
        page.contains(&format!(
            "<form class=\"remark-form\" method=\"post\" action=\"/topics/{SUBJECT_ID}/comments/{REMARK_ID}/edit\">"
        )),
        "{page}"
    );
    assert!(page.contains(">a remark on it</textarea>"), "{page}");
    assert!(page.contains("name=\"token\""), "{page}");
}

#[tokio::test]
async fn a_remark_changed_from_the_page_goes_to_the_board_with_the_session() {
    let (board_url, log) = board(SUBJECT, REMARK, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/edit"),
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/edit"),
        &[("body", "a remark on it, mended")],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}#remark-{REMARK_ID}")
    );
    let calls = log.calls();
    assert!(
        calls.contains(&format!(
            "PATCH /api/topics/{SUBJECT_ID}/comments/{REMARK_ID}"
        )),
        "{calls:?}"
    );
}

#[tokio::test]
async fn a_remark_nobody_knows_is_refused() {
    let (board_url, _) = board(SUBJECT, REMARK, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/comments/99999999-9999-9999-9999-999999999999/edit"),
        Some("token"),
    )
    .await;
    assert_eq!(answer.status(), 404);
}

#[tokio::test]
async fn the_page_to_remove_a_subject_carries_the_way_to_say_why() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/remove"),
        Some("token"),
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(page.contains("<h2>Remove a subject</h2>"), "{page}");
    assert!(
        page.contains(&format!(
            "<form class=\"removal-form\" method=\"post\" action=\"/topics/{SUBJECT_ID}/delete\">"
        )),
        "{page}"
    );
    assert!(page.contains("name=\"reason\""), "{page}");
    assert!(page.contains("name=\"token\""), "{page}");
}

#[tokio::test]
async fn a_removal_written_from_the_page_goes_to_the_board_with_the_session() {
    let (board_url, log) = board(SUBJECT, REMOVED, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/remove"),
        &format!("/topics/{SUBJECT_ID}/delete"),
        &[("reason", "off topic")],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}")
    );
    let calls = log.calls();
    assert!(
        calls.contains(&format!("POST /api/topics/{SUBJECT_ID}/delete")),
        "{calls:?}"
    );
    let body = calls
        .iter()
        .find(|call| call.starts_with("BODY {"))
        .expect("the board is written to with a body");
    assert!(body.contains("\"reason\":\"off topic\""), "{body}");
}

#[tokio::test]
async fn a_removal_written_without_the_token_of_its_page_is_refused() {
    let (board_url, log) = board(SUBJECT, REMOVED, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = sent(
        &base,
        &format!("/topics/{SUBJECT_ID}/delete"),
        &[("reason", "off topic")],
    )
    .await;
    assert_eq!(answer.status(), 403);
    assert!(
        !log.calls()
            .contains(&format!("POST /api/topics/{SUBJECT_ID}/delete")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_subject_that_was_removed_is_brought_back_from_the_page() {
    let (board_url, log) = board(REMOVED, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/restore"),
        Some("token"),
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(page.contains("<h2>Bring a subject back</h2>"), "{page}");
    assert!(
        page.contains(&format!(
            "<form class=\"restore-form\" method=\"post\" action=\"/topics/{SUBJECT_ID}/restore\">"
        )),
        "{page}"
    );
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/restore"),
        &format!("/topics/{SUBJECT_ID}/restore"),
        &[],
    )
    .await;
    assert_eq!(answer.status(), 303);
    let calls = log.calls();
    assert!(
        calls.contains(&format!("POST /api/topics/{SUBJECT_ID}/restore")),
        "{calls:?}"
    );
}

#[tokio::test]
async fn a_remark_is_removed_and_brought_back_from_the_page() {
    let (board_url, log) = board(SUBJECT, REMARK, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/remove"),
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/delete"),
        &[("reason", "off topic")],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}#remark-{REMARK_ID}")
    );
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/restore"),
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/restore"),
        &[],
    )
    .await;
    assert_eq!(answer.status(), 303);
    let calls = log.calls();
    assert!(
        calls.contains(&format!(
            "POST /api/topics/{SUBJECT_ID}/comments/{REMARK_ID}/delete"
        )),
        "{calls:?}"
    );
    assert!(
        calls.contains(&format!(
            "POST /api/topics/{SUBJECT_ID}/comments/{REMARK_ID}/restore"
        )),
        "{calls:?}"
    );
}

#[tokio::test]
async fn the_history_page_carries_the_versions_of_a_subject() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}/history"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>What changed</h2>"), "{page}");
    assert!(page.contains("alice"), "{page}");
    assert!(page.contains("bob"), "{page}");
    assert!(
        page.contains(&format!(
            "<a href=\"/topics/{SUBJECT_ID}/history/{VERSION_ID}\">"
        )),
        "{page}"
    );
    assert!(
        page.contains(&format!("<a href=\"/topics/{SUBJECT_ID}\">")),
        "{page}"
    );
}

#[tokio::test]
async fn a_subject_nothing_was_changed_in_says_so() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", NO_VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}/history"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("Nothing was changed here yet."), "{page}");
}

#[tokio::test]
async fn a_version_page_carries_what_changed_in_it() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/history/{VERSION_ID}"),
        None,
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(
        page.contains("<h2>What changed at 2024-06-07 10:11</h2>"),
        "{page}"
    );
    assert!(page.contains("the body of it, mended"), "{page}");
    assert!(page.contains("added"), "{page}");
    assert!(page.contains("removed"), "{page}");
    assert!(
        page.contains(&format!("<a href=\"/topics/{SUBJECT_ID}/history\">")),
        "{page}"
    );
}

#[tokio::test]
async fn a_version_nobody_knows_is_refused() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let answer = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/history/99999999-9999-9999-9999-999999999999"),
        None,
    )
    .await;
    assert_eq!(answer.status(), 404);
}

#[tokio::test]
async fn a_subject_offers_the_ways_to_change_it_and_to_remove_it() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        page.contains(&format!("<a href=\"/topics/{SUBJECT_ID}/edit\">Change</a>")),
        "{page}"
    );
    assert!(
        page.contains(&format!(
            "<a href=\"/topics/{SUBJECT_ID}/remove\">Remove</a>"
        )),
        "{page}"
    );
    assert!(
        page.contains(&format!(
            "<a href=\"/topics/{SUBJECT_ID}/comments/{REMARK_ID}/edit\">Change</a>"
        )),
        "{page}"
    );
    assert!(
        page.contains(&format!(
            "<a href=\"/topics/{SUBJECT_ID}/comments/{REMARK_ID}/remove\">Remove</a>"
        )),
        "{page}"
    );
    assert!(
        page.contains(&format!("<a href=\"/topics/{SUBJECT_ID}/history\">")),
        "{page}"
    );
}

#[tokio::test]
async fn a_subject_that_was_removed_offers_the_way_to_bring_it_back() {
    let (board_url, _) = board(REMOVED, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        page.contains(&format!(
            "<a href=\"/topics/{SUBJECT_ID}/restore\">Bring back</a>"
        )),
        "{page}"
    );
}

#[tokio::test]
async fn a_reader_with_no_account_is_offered_no_way_to_change_what_was_written() {
    let (board_url, _) = board(SUBJECT, SUBJECT, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(!page.contains("/edit"), "{page}");
    assert!(!page.contains("/remove"), "{page}");
    assert!(
        page.contains(&format!("<a href=\"/topics/{SUBJECT_ID}/history\">")),
        "{page}"
    );
}

#[tokio::test]
async fn the_pages_of_changing_carry_no_scripting() {
    let (board_url, _) = board(SUBJECT, REMARK, "200 OK", VERSIONS).await;
    let base = front(&board_url).await;
    for address in [
        format!("/topics/{SUBJECT_ID}/edit"),
        format!("/topics/{SUBJECT_ID}/remove"),
        format!("/topics/{SUBJECT_ID}/restore"),
        format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/edit"),
        format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/remove"),
        format!("/topics/{SUBJECT_ID}/history"),
        format!("/topics/{SUBJECT_ID}/history/{VERSION_ID}"),
    ] {
        let page = get(&base, &address, Some("token"))
            .await
            .text()
            .await
            .unwrap();
        assert!(front::html::scripting_free(&page), "{address}: {page}");
    }
}
