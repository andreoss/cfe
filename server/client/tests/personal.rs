use client::{ApiClient, ClientError};
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

const SUBJECTS: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"title\":\"Ports and adapters\",",
    "\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",",
    "\"tags\":[\"rust\"],\"sticky\":false,\"resolved\":false,\"deleted\":false,",
    "\"pending\":false,\"draft\":false,\"postscore\":2}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":3,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const EMPTY: &str = concat!(
    "{\"items\":[],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":0,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const NOTIFICATIONS: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"22222222-2222-2222-2222-222222222222\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"topic_title\":\"Ports and adapters\",",
    "\"comment_id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"actor_username\":\"bob\",\"created_at\":\"2024-06-08T09:08:07Z\",",
    "\"read\":false,\"kind\":\"reply\"}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const TAGS: &str = "[\"rust\",\"adapters\"]";
const UNREAD: &str = "{\"unread\":4}";
const JSON: &[&str] = &["content-type: application/json"];

#[tokio::test]
async fn the_bookmarks_of_an_account_are_asked_for_with_its_session() {
    let (base, log) = board(answer("200 OK", JSON, SUBJECTS)).await;
    let client = ApiClient::new(&base).unwrap();
    client.bookmarks("token", 2).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("GET /api/bookmarks?page=2 "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn what_an_account_watches_is_asked_for_with_its_session() {
    let (base, log) = board(answer("200 OK", JSON, SUBJECTS)).await;
    let client = ApiClient::new(&base).unwrap();
    client.watched("token", 1).await.unwrap();
    let request = log.last();
    assert!(request.starts_with("GET /api/watched?page=1 "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_bookmarks_and_the_watched_come_back_as_pages_of_subjects() {
    let (base, _) = board(answer("200 OK", JSON, SUBJECTS)).await;
    let client = ApiClient::new(&base).unwrap();
    let bookmarks = client.bookmarks("token", 1).await.unwrap();
    assert_eq!(bookmarks.items.len(), 1);
    assert_eq!(bookmarks.items[0].title, "Ports and adapters".to_owned());
    assert_eq!(bookmarks.page.total, 3);
    let watched = client.watched("token", 1).await.unwrap();
    assert_eq!(watched.items.len(), 1);
}

#[tokio::test]
async fn an_account_that_kept_nothing_comes_back_empty() {
    let (base, _) = board(answer("200 OK", JSON, EMPTY)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.bookmarks("token", 1).await.unwrap().items.is_empty());
    assert!(client.watched("token", 1).await.unwrap().items.is_empty());
}

#[tokio::test]
async fn the_tags_an_account_follows_are_asked_for_with_its_session() {
    let (base, log) = board(answer("200 OK", JSON, TAGS)).await;
    let client = ApiClient::new(&base).unwrap();
    let tags = client.followed_tags("token").await.unwrap();
    assert_eq!(tags, vec!["rust".to_owned(), "adapters".to_owned()]);
    let request = log.last();
    assert!(request.starts_with("GET /api/followed-tags "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_notifications_of_an_account_come_back_with_what_they_name() {
    let (base, log) = board(answer("200 OK", JSON, NOTIFICATIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    let page = client.notifications("token", 1).await.unwrap();
    assert_eq!(page.items.len(), 1);
    let notice = &page.items[0];
    assert_eq!(notice.id, "22222222-2222-2222-2222-222222222222");
    assert_eq!(notice.topic_title, "Ports and adapters".to_owned());
    assert_eq!(notice.actor_username, "bob".to_owned());
    assert_eq!(notice.kind, "reply".to_owned());
    assert!(!notice.read);
    let request = log.last();
    assert!(
        request.starts_with("GET /api/notifications?page=1 "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn how_much_an_account_has_not_read_yet_is_asked_for_by_its_address() {
    let (base, log) = board(answer("200 OK", JSON, UNREAD)).await;
    let client = ApiClient::new(&base).unwrap();
    let count = client.unread_count("token").await.unwrap();
    assert_eq!(count.unread, 4);
    let request = log.last();
    assert!(
        request.starts_with("GET /api/notifications/unread-count "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn marking_a_notification_read_is_a_post_of_its_own() {
    let (base, log) = board(answer("200 OK", JSON, "{}")).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .mark_read("token", "22222222-2222-2222-2222-222222222222")
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/notifications/22222222-2222-2222-2222-222222222222/read "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_board_that_refuses_what_an_account_kept_keeps_its_reason() {
    let (base, _) = board(answer(
        "401 Unauthorized",
        JSON,
        "{\"error\":\"not signed in\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.bookmarks("token", 1).await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 401);
            assert_eq!(reason, "not signed in");
        }
        other => panic!("a refusal must keep its reason: {other:?}"),
    }
}

#[tokio::test]
async fn an_answer_that_is_no_json_is_an_error_and_not_a_guess() {
    let (base, _) = board(answer("200 OK", JSON, "not json")).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.followed_tags("token").await,
        Err(ClientError::Detail(_))
    ));
}
