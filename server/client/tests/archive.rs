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

const MONTHS: &str =
    "[{\"year\":2024,\"month\":6,\"topics\":3},{\"year\":2024,\"month\":5,\"topics\":1}]";

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

const ACTIVITY: &str = concat!(
    "[",
    "{\"kind\":\"topic\",\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"title\":\"Ports and adapters\",",
    "\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",",
    "\"tags\":[\"rust\"],\"sticky\":false,\"resolved\":false,\"deleted\":false,",
    "\"pending\":false,\"draft\":false,\"postscore\":2},",
    "{\"kind\":\"comment\",\"id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,",
    "\"body\":\"Adapters keep the domain clean\",\"author_username\":\"bob\",",
    "\"created_at\":\"2024-06-08T09:08:07Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"ignored\":false}",
    "]"
);

const JSON: &[&str] = &["content-type: application/json"];

#[tokio::test]
async fn the_months_of_the_archive_are_asked_for_by_their_address() {
    let (base, log) = board(answer("200 OK", JSON, MONTHS)).await;
    let client = ApiClient::new(&base).unwrap();
    client.archive(None).await.unwrap();
    let request = log.last();
    assert!(request.starts_with("GET /api/archive "), "{request}");
    assert!(!request.contains("cookie:"), "{request}");
}

#[tokio::test]
async fn a_month_of_the_archive_is_asked_for_by_its_year_its_month_and_a_page() {
    let (base, log) = board(answer("200 OK", JSON, SUBJECTS)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .archive_month(2024, 6, 2, Some("token"))
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with("GET /api/archive/2024/6?page=2 "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_months_keep_their_year_their_month_and_how_many_they_carry() {
    let (base, _) = board(answer("200 OK", JSON, MONTHS)).await;
    let client = ApiClient::new(&base).unwrap();
    let months = client.archive(None).await.unwrap();
    assert_eq!(months.len(), 2);
    assert_eq!(months[0].year, 2024);
    assert_eq!(months[0].month, 6);
    assert_eq!(months[0].topics, 3);
    assert_eq!(months[1].topics, 1);
}

#[tokio::test]
async fn a_month_answers_with_a_page_of_its_subjects() {
    let (base, _) = board(answer("200 OK", JSON, SUBJECTS)).await;
    let client = ApiClient::new(&base).unwrap();
    let page = client.archive_month(2024, 6, 1, None).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].title, "Ports and adapters".to_owned());
    assert_eq!(page.page.total, 3);
}

#[tokio::test]
async fn what_was_written_and_said_last_comes_back_as_subjects_and_remarks() {
    let (base, log) = board(answer("200 OK", JSON, ACTIVITY)).await;
    let client = ApiClient::new(&base).unwrap();
    let items = client.activity(Some("token")).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].kind(), "topic");
    assert_eq!(
        items[0].subject().map(|topic| topic.title.as_str()),
        Some("Ports and adapters")
    );
    assert_eq!(items[1].kind(), "comment");
    assert_eq!(
        items[1].remark().map(|remark| remark.body.as_str()),
        Some("Adapters keep the domain clean")
    );
    let request = log.last();
    assert!(request.starts_with("GET /api/activity "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_month_nobody_wrote_in_comes_back_empty() {
    let (base, _) = board(answer("200 OK", JSON, "[]")).await;
    let client = ApiClient::new(&base).unwrap();
    let months = client.archive(None).await.unwrap();
    assert!(months.is_empty());
}

#[tokio::test]
async fn a_month_the_board_refuses_keeps_its_reason() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        JSON,
        "{\"error\":\"no such month\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.archive_month(2024, 13, 1, None).await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 422);
            assert_eq!(reason, "no such month");
        }
        other => panic!("a refused month must keep its reason: {other:?}"),
    }
}
