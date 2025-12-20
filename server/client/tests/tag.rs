use client::{ApiClient, ClientError, Tag};
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

const TAG: &str = concat!(
    "{\"slug\":\"rust\",\"description\":\"The rust language\",",
    "\"means\":\"systems\",\"following\":true}"
);

const BARE_TAG: &str =
    "{\"slug\":\"rust\",\"description\":null,\"means\":null,\"following\":false}";

const SUBJECTS: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"title\":\"Ports and adapters\",",
    "\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",",
    "\"tags\":[\"rust\"],\"sticky\":false,\"resolved\":false,\"deleted\":false,",
    "\"pending\":false,\"draft\":false,\"postscore\":2}],",
    "\"page\":{\"number\":2,\"size\":20,\"total\":21,\"total_pages\":2,",
    "\"has_next\":false,\"has_previous\":true}}"
);

const NO_SUBJECTS: &str = concat!(
    "{\"items\":[],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":0,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

#[tokio::test]
async fn a_tag_is_asked_for_by_its_address() {
    let (base, log) = board(answer("200 OK", &["content-type: application/json"], TAG)).await;
    let client = ApiClient::new(&base).unwrap();
    client.tag("rust", None).await.unwrap();
    let request = log.last();
    assert!(request.starts_with("GET /api/tags/rust "), "{request}");
}

#[tokio::test]
async fn the_tag_stays_inside_one_step_of_the_address() {
    let (base, log) = board(answer("200 OK", &["content-type: application/json"], TAG)).await;
    let client = ApiClient::new(&base).unwrap();
    client.tag("a/b?c d", None).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("GET /api/tags/a%2Fb%3Fc%20d "),
        "{request}"
    );
}

#[tokio::test]
async fn a_tag_is_asked_for_with_the_session_when_one_is_held() {
    let (base, log) = board(answer("200 OK", &["content-type: application/json"], TAG)).await;
    let client = ApiClient::new(&base).unwrap();
    client.tag("rust", Some("token")).await.unwrap();
    assert!(
        log.last().contains("cookie: session=token"),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn a_tag_carries_what_it_is_for_what_it_means_and_whether_it_is_followed() {
    let (base, _) = board(answer("200 OK", &["content-type: application/json"], TAG)).await;
    let client = ApiClient::new(&base).unwrap();
    let tag = client.tag("rust", None).await.unwrap();
    assert_eq!(tag.slug, "rust".to_owned());
    assert_eq!(tag.description.as_deref(), Some("The rust language"));
    assert_eq!(tag.means.as_deref(), Some("systems"));
    assert!(tag.following);
}

#[tokio::test]
async fn a_tag_nobody_wrote_words_for_comes_back_without_them() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        BARE_TAG,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let tag = client.tag("rust", None).await.unwrap();
    assert_eq!(tag.description, None);
    assert_eq!(tag.means, None);
    assert!(!tag.following);
}

#[tokio::test]
async fn a_tag_the_board_refuses_keeps_its_reason() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        &["content-type: application/json"],
        "{\"error\":\"invalid tag\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.tag("rust!", None).await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 422);
            assert_eq!(reason, "invalid tag");
        }
        other => panic!("a refused tag must keep its reason: {other:?}"),
    }
}

#[tokio::test]
async fn an_unreadable_tag_is_an_error() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        "not json",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.tag("rust", None).await {
        Err(ClientError::Detail(_)) => {}
        other => panic!("an unreadable answer must be an error: {other:?}"),
    }
}

#[tokio::test]
async fn the_subjects_of_a_tag_are_asked_for_a_page_at_a_time() {
    let (base, log) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        SUBJECTS,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    client.tag_topics("rust", 2).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("GET /api/tags/rust/topics?page=2 "),
        "{request}"
    );
}

#[tokio::test]
async fn a_page_of_subjects_comes_back_with_the_subjects_and_the_pager() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        SUBJECTS,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let page = client.tag_topics("rust", 2).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].title, "Ports and adapters".to_owned());
    assert_eq!(page.items[0].tags, vec!["rust".to_owned()]);
    assert_eq!(page.page.number, 2);
    assert_eq!(page.page.total, 21);
    assert!(page.page.has_previous);
    assert!(!page.page.has_next);
}

#[tokio::test]
async fn a_tag_nobody_used_answers_with_an_empty_page() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        NO_SUBJECTS,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let page = client.tag_topics("rust", 1).await.unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.page.total, 0);
}

#[test]
fn a_tag_keeps_the_address_it_was_asked_for_and_the_words_it_was_given() {
    let tag = Tag {
        slug: "rust".to_owned(),
        description: Some("The rust language".to_owned()),
        means: None,
        following: false,
    };
    assert_eq!(tag.slug, "rust".to_owned());
    assert_eq!(tag.description.as_deref(), Some("The rust language"));
    assert_eq!(tag.means, None);
    assert!(!tag.following);
}
