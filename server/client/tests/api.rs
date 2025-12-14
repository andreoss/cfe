use client::{ApiClient, ClientError, Section};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn targets(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn spawn_stub(
    status: &'static str,
    content_type: &'static str,
    body: &'static str,
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
                let mut buffer = vec![0u8; 4096];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                if let Some(target) = request.split_whitespace().nth(1) {
                    sink.lock().unwrap().push(target.to_owned());
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    (format!("http://{address}"), log)
}

async fn stub(status: &'static str, content_type: &'static str, body: &'static str) -> String {
    spawn_stub(status, content_type, body).await.0
}

async fn closed_port() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}")
}

#[tokio::test]
async fn the_section_list_is_read_from_the_api() {
    let base = stub("200 OK", "application/json", "[{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"anyone\",\"may_post\":true}]").await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.sections().await.unwrap(),
        vec![Section {
            slug: "general".to_owned(),
            title: "General Talk".to_owned(),
            topics_score: "anyone".to_owned(),
            may_post: true,
        }]
    );
}

#[tokio::test]
async fn an_empty_list_is_an_empty_page() {
    let base = stub("200 OK", "application/json", "[]").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.sections().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_status_outside_the_success_range_is_an_error() {
    let base = stub("503 Service Unavailable", "application/json", "{}").await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.sections().await.unwrap_err(),
        ClientError::Status(503)
    );
}

#[tokio::test]
async fn an_answer_that_is_not_the_promised_shape_is_an_error() {
    let base = stub("200 OK", "application/json", "{\"slug\":\"general\"}").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.sections().await.unwrap_err(),
        ClientError::Detail(_)
    ));
}

#[tokio::test]
async fn an_answer_that_is_not_json_is_an_error() {
    let base = stub("200 OK", "text/html", "<html>nope</html>").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.sections().await.unwrap_err(),
        ClientError::Detail(_)
    ));
}

#[tokio::test]
async fn a_server_that_is_not_there_is_reported_not_raised() {
    let client = ApiClient::new(&closed_port().await).unwrap();
    assert!(matches!(
        client.sections().await.unwrap_err(),
        ClientError::Transport(_)
    ));
}

const TOPICS: &str = "{\"items\":[{\"id\":\"11111111-1111-1111-1111-111111111111\",\"section_slug\":\"general\",\"title\":\"First\",\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",\"tags\":[\"rust\"],\"sticky\":true,\"resolved\":false,\"deleted\":false,\"pending\":false,\"draft\":false,\"postscore\":3}],\"page\":{\"number\":1,\"size\":25,\"total\":30,\"total_pages\":2,\"has_next\":true,\"has_previous\":false}}";

#[tokio::test]
async fn a_page_of_subjects_is_read_from_the_api() {
    let base = stub("200 OK", "application/json", TOPICS).await;
    let client = ApiClient::new(&base).unwrap();
    let page = client.topics("general", 1).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].title, "First");
    assert_eq!(page.items[0].author_username, "alice");
    assert_eq!(page.items[0].tags, vec!["rust".to_owned()]);
    assert!(page.items[0].sticky);
    assert!(!page.items[0].resolved);
    assert_eq!(page.items[0].postscore, 3);
    assert_eq!(page.page.number, 1);
    assert_eq!(page.page.total, 30);
    assert_eq!(page.page.total_pages, 2);
    assert!(page.page.has_next);
    assert!(!page.page.has_previous);
}

#[tokio::test]
async fn the_page_number_is_put_in_the_address() {
    let (base, log) = spawn_stub("200 OK", "application/json", TOPICS).await;
    let client = ApiClient::new(&base).unwrap();
    let _ = client.topics("general", 3).await.unwrap();
    assert_eq!(log.targets(), vec!["/api/sections/general/topics?page=3"]);
}

#[tokio::test]
async fn an_odd_slug_cannot_leave_the_address_it_belongs_to() {
    let (base, log) = spawn_stub("200 OK", "application/json", TOPICS).await;
    let client = ApiClient::new(&base).unwrap();
    let _ = client.topics("a/b?c d", 1).await.unwrap();
    assert_eq!(
        log.targets(),
        vec!["/api/sections/a%2Fb%3Fc%20d/topics?page=1"]
    );
}

#[tokio::test]
async fn a_section_with_no_subjects_is_an_empty_page() {
    let base = stub(
        "200 OK",
        "application/json",
        "{\"items\":[],\"page\":{\"number\":1,\"size\":25,\"total\":0,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}",
    )
    .await;
    let client = ApiClient::new(&base).unwrap();
    let page = client.topics("general", 1).await.unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.page.total, 0);
}

#[tokio::test]
async fn a_section_that_is_not_there_is_reported_as_a_status() {
    let base = stub("404 Not Found", "application/json", "{\"error\":\"no\"}").await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.topics("nope", 1).await.unwrap_err(),
        ClientError::Status(404)
    );
}

#[tokio::test]
async fn a_page_that_is_not_the_promised_shape_is_an_error() {
    let base = stub("200 OK", "application/json", "{\"items\":[]}").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.topics("general", 1).await.unwrap_err(),
        ClientError::Detail(_)
    ));
}

const SUBJECT: &str = "{\"id\":\"11111111-1111-1111-1111-111111111111\",\"section_slug\":\"general\",\"group_slug\":null,\"title\":\"First subject\",\"body\":\"The opening remark\\nwith two lines\",\"tags\":[\"rust\"],\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"postscore\":2,\"pending\":false,\"draft\":false,\"sticky\":true,\"off_front\":false,\"resolved\":false,\"minor\":false,\"open_reports\":0}";

const COMMENTS: &str = "{\"items\":[{\"id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"First reply\",\"author_username\":\"bob\",\"created_at\":\"2024-06-07T11:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false},{\"id\":\"bbbbbbbb-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"body\":\"A reply to the first\",\"author_username\":\"carol\",\"created_at\":\"2024-06-07T12:00:00Z\",\"deleted\":true,\"deleted_reason\":\"off topic\",\"edited\":true,\"ignored\":false}],\"page\":{\"number\":1,\"size\":25,\"total\":2,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}";

#[tokio::test]
async fn a_subject_is_read_from_the_api() {
    let (base, log) = spawn_stub("200 OK", "application/json", SUBJECT).await;
    let client = ApiClient::new(&base).unwrap();
    let subject = client
        .subject("11111111-1111-1111-1111-111111111111")
        .await
        .unwrap();
    assert_eq!(
        log.targets(),
        vec!["/api/topics/11111111-1111-1111-1111-111111111111"]
    );
    assert_eq!(subject.title, "First subject");
    assert_eq!(subject.body, "The opening remark\nwith two lines");
    assert_eq!(subject.section_slug, "general");
    assert_eq!(subject.author_username, "alice");
    assert_eq!(subject.tags, vec!["rust".to_owned()]);
    assert!(subject.sticky);
    assert!(!subject.deleted);
    assert_eq!(subject.postscore, 2);
}

#[tokio::test]
async fn a_subject_that_is_not_there_is_reported_as_a_status() {
    let base = stub("404 Not Found", "application/json", "{\"error\":\"no\"}").await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.subject("nope").await.unwrap_err(),
        ClientError::Status(404)
    );
}

#[tokio::test]
async fn the_remarks_under_a_subject_are_read_from_the_api() {
    let (base, log) = spawn_stub("200 OK", "application/json", COMMENTS).await;
    let client = ApiClient::new(&base).unwrap();
    let page = client
        .comments("11111111-1111-1111-1111-111111111111", 2)
        .await
        .unwrap();
    assert_eq!(
        log.targets(),
        vec!["/api/topics/11111111-1111-1111-1111-111111111111/comments?page=2"]
    );
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].body, "First reply");
    assert_eq!(page.items[0].author_username, "bob");
    assert_eq!(page.items[1].parent_id, Some(page.items[0].id.clone()));
    assert!(page.items[1].deleted);
    assert_eq!(page.items[1].deleted_reason, Some("off topic".to_owned()));
    assert!(page.items[1].edited);
    assert_eq!(page.page.total, 2);
}

#[tokio::test]
async fn a_subject_with_no_remarks_is_an_empty_page() {
    let base = stub(
        "200 OK",
        "application/json",
        "{\"items\":[],\"page\":{\"number\":1,\"size\":25,\"total\":0,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}",
    )
    .await;
    let client = ApiClient::new(&base).unwrap();
    let page = client
        .comments("11111111-1111-1111-1111-111111111111", 1)
        .await
        .unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.page.total, 0);
}

#[tokio::test]
async fn a_subject_that_is_not_the_promised_shape_is_an_error() {
    let base = stub("200 OK", "application/json", "{\"id\":\"1\"}").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.subject("1").await.unwrap_err(),
        ClientError::Detail(_)
    ));
    assert!(matches!(
        client.comments("1", 1).await.unwrap_err(),
        ClientError::Detail(_)
    ));
}

#[tokio::test]
async fn a_section_is_found_among_the_sections() {
    let base = stub("200 OK", "application/json", "[{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"anyone\",\"may_post\":true},{\"slug\":\"news\",\"title\":\"News\",\"topics_score\":\"moderators\",\"may_post\":false}]").await;
    let client = ApiClient::new(&base).unwrap();
    let sections = client.sections().await.unwrap();
    assert_eq!(
        client::find_section(&sections, "news").map(|section| section.title.clone()),
        Some("News".to_owned())
    );
    assert_eq!(client::find_section(&sections, "nope"), None);
}
