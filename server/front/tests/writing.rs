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

const SECTIONS: &str =
    "[{\"slug\":\"general\",\"title\":\"General\",\"topics_score\":\"0\",\"may_post\":true}]";

const USER: &str = "{\"id\":\"9\",\"username\":\"alice\",\"role\":\"user\"}";
const REFUSED: &str = "{\"error\":\"not allowed to post here\"}";

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const REMARK_ID: &str = "33333333-3333-3333-3333-333333333333";

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(written: &'static str, reason: &'static str) -> (String, Log) {
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
                    "/api/sections" => ("200 OK", SECTIONS),
                    "/api/sections/general/topics" => (reason, written),
                    "/api/topics/11111111-1111-1111-1111-111111111111" => ("200 OK", SUBJECT),
                    "/api/topics/11111111-1111-1111-1111-111111111111/comments" => {
                        match method.as_str() {
                            "POST" => (reason, written),
                            _ => ("200 OK", REMARKS),
                        }
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

#[tokio::test]
async fn the_page_to_write_a_subject_is_given_to_an_account() {
    let (board_url, _) = board(SUBJECT, "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, "/sections/general/post", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Write a subject</h2>"), "{page}");
    assert!(
        page.contains(
            "<form class=\"subject-form\" method=\"post\" action=\"/sections/general/post\">"
        ),
        "{page}"
    );
    assert!(page.contains("name=\"title\""), "{page}");
    assert!(page.contains("name=\"body\""), "{page}");
    assert!(page.contains("name=\"tags\""), "{page}");
    assert!(page.contains("name=\"draft\""), "{page}");
    assert!(page.contains("name=\"token\""), "{page}");
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in_before_writing() {
    let (board_url, _) = board(SUBJECT, "200 OK").await;
    let base = front(&board_url).await;
    let answer = get(&base, "/sections/general/post", None).await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        "/sign-in?return_to=/sections/general/post"
    );
}

#[tokio::test]
async fn a_subject_written_from_the_page_goes_to_the_board_with_the_session() {
    let (board_url, log) = board(SUBJECT, "200 OK").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        "/sections/general/post",
        "/sections/general/post",
        &[
            ("title", "Ports and adapters"),
            ("body", "the body of it"),
            ("tags", "rust, adapters"),
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
        calls.contains(&"POST /api/sections/general/topics".to_owned()),
        "{calls:?}"
    );
    let body = calls
        .iter()
        .find(|call| call.starts_with("BODY {"))
        .expect("the board is written to with a body");
    assert!(body.contains("\"title\":\"Ports and adapters\""), "{body}");
    assert!(body.contains("\"tags\":[\"rust\",\"adapters\"]"), "{body}");
}

#[tokio::test]
async fn a_subject_written_without_the_token_of_its_page_is_refused() {
    let (board_url, log) = board(SUBJECT, "200 OK").await;
    let base = front(&board_url).await;
    let answer = http()
        .post(format!("{base}/sections/general/post"))
        .header("cookie", "session=token")
        .form(&[("title", "Ports"), ("body", "the body")])
        .send()
        .await
        .unwrap();
    assert_eq!(answer.status(), 403);
    assert!(
        !log.calls()
            .contains(&"POST /api/sections/general/topics".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_board_that_refuses_a_subject_keeps_its_reason() {
    let (board_url, _) = board(REFUSED, "403 Forbidden").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        "/sections/general/post",
        "/sections/general/post",
        &[("title", "Ports"), ("body", "the body")],
    )
    .await;
    assert_eq!(answer.status(), 422);
    let page = answer.text().await.unwrap();
    assert!(page.contains("not allowed to post here"), "{page}");
}

#[tokio::test]
async fn the_page_to_answer_a_subject_is_given_to_an_account() {
    let (board_url, _) = board(REMARK, "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}/reply"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Answer</h2>"), "{page}");
    assert!(
        page.contains(&format!(
            "<form class=\"remark-form\" method=\"post\" action=\"/topics/{SUBJECT_ID}/comments\">"
        )),
        "{page}"
    );
    assert!(page.contains("name=\"body\""), "{page}");
    assert!(page.contains("name=\"token\""), "{page}");
    assert!(page.contains("Ports and adapters"), "{page}");
}

#[tokio::test]
async fn an_answer_to_a_remark_carries_the_remark_it_answers() {
    let (board_url, _) = board(REMARK, "200 OK").await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/reply?parent={REMARK_ID}"),
        Some("token"),
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(
        page.contains(&format!(
            "<input type=\"hidden\" name=\"parent_id\" value=\"{REMARK_ID}\">"
        )),
        "{page}"
    );
    assert!(page.contains("alice"), "{page}");
    assert!(page.contains("a remark on it"), "{page}");
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in_before_answering() {
    let (board_url, _) = board(REMARK, "200 OK").await;
    let base = front(&board_url).await;
    let answer = get(&base, &format!("/topics/{SUBJECT_ID}/reply"), None).await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/sign-in?return_to=/topics/{SUBJECT_ID}/reply")
    );
}

#[tokio::test]
async fn an_answer_written_from_the_page_goes_to_the_board_with_the_session() {
    let (board_url, log) = board(REMARK, "200 OK").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/reply"),
        &format!("/topics/{SUBJECT_ID}/comments"),
        &[("body", "a remark on it")],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}#remark-{REMARK_ID}")
    );
    let calls = log.calls();
    assert!(
        calls.contains(&format!("POST /api/topics/{SUBJECT_ID}/comments")),
        "{calls:?}"
    );
}

#[tokio::test]
async fn an_answer_written_without_the_token_of_its_page_is_refused() {
    let (board_url, log) = board(REMARK, "200 OK").await;
    let base = front(&board_url).await;
    let answer = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/comments"))
        .header("cookie", "session=token")
        .form(&[("body", "a remark on it")])
        .send()
        .await
        .unwrap();
    assert_eq!(answer.status(), 403);
    assert!(
        !log.calls()
            .contains(&format!("POST /api/topics/{SUBJECT_ID}/comments")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_subject_offers_the_way_to_answer_it() {
    let (board_url, _) = board(REMARK, "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        page.contains(&format!(
            "<a href=\"/topics/{SUBJECT_ID}/reply\">Answer</a>"
        )),
        "{page}"
    );
}
