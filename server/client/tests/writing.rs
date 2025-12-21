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

const HISTORY: &str = concat!(
    "[{\"id\":\"44444444-4444-4444-4444-444444444444\",",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"editor\":\"alice\",\"written_at\":\"2024-06-07T10:11:12Z\"}]"
);

const CHANGES: &str = concat!(
    "[{\"kind\":\"kept\",\"line\":\"the same line\"},",
    "{\"kind\":\"removed\",\"line\":\"a line taken away\"},",
    "{\"kind\":\"added\",\"line\":\"a line put in\"}]"
);

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const REMARK_ID: &str = "33333333-3333-3333-3333-333333333333";
const VERSION_ID: &str = "44444444-4444-4444-4444-444444444444";

#[tokio::test]
async fn a_subject_is_written_to_its_section_with_the_session() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    let written = client
        .create_topic(
            "token",
            "general",
            &client::NewSubject::new("Ports", "the body"),
        )
        .await
        .unwrap();
    assert_eq!(written.id, SUBJECT_ID.to_owned());
    let request = log.last();
    assert!(
        request.starts_with("POST /api/sections/general/topics "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"title\":\"Ports\""), "{request}");
    assert!(request.contains("\"body\":\"the body\""), "{request}");
}

#[tokio::test]
async fn a_subject_is_written_with_its_tags_and_an_answer_to_a_challenge() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .create_topic(
            "token",
            "general",
            &client::NewSubject::new("Ports", "the body")
                .tags(&["rust", "adapters"])
                .group("team")
                .challenge("seven")
                .draft(),
        )
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.contains("\"tags\":[\"rust\",\"adapters\"]"),
        "{request}"
    );
    assert!(request.contains("\"group\":\"team\""), "{request}");
    assert!(request.contains("\"challenge\":\"seven\""), "{request}");
    assert!(request.contains("\"draft\":true"), "{request}");
}

#[tokio::test]
async fn a_section_that_is_not_there_is_refused_with_its_reason() {
    let (base, _) = board(answer("404 Not Found", "{\"error\":\"section not found\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    match client
        .create_topic("token", "gone", &client::NewSubject::new("Ports", "body"))
        .await
    {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 404);
            assert_eq!(reason, "section not found");
        }
        other => panic!("a refusal must keep its reason: {other:?}"),
    }
}

#[tokio::test]
async fn a_remark_is_written_to_a_subject_with_the_session() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    let written = client
        .create_comment("token", SUBJECT_ID, &client::NewRemark::new("a remark"))
        .await
        .unwrap();
    assert_eq!(written.id, REMARK_ID.to_owned());
    assert_eq!(written.body, "a remark on it".to_owned());
    let request = log.last();
    assert!(
        request.starts_with(&format!("POST /api/topics/{SUBJECT_ID}/comments ")),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"body\":\"a remark\""), "{request}");
}

#[tokio::test]
async fn a_remark_that_answers_another_names_it_as_its_parent() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .create_comment(
            "token",
            SUBJECT_ID,
            &client::NewRemark::new("an answer").reply_to(REMARK_ID),
        )
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.contains(&format!("\"parent_id\":\"{REMARK_ID}\"")),
        "{request}"
    );
}

#[tokio::test]
async fn a_remark_written_with_no_parent_names_none() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .create_comment("token", SUBJECT_ID, &client::NewRemark::new("a remark"))
        .await
        .unwrap();
    let request = log.last();
    assert!(request.contains("\"parent_id\":null"), "{request}");
}

#[tokio::test]
async fn a_subject_is_changed_with_a_patch_of_its_own() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    let changed = client
        .edit_topic(
            "token",
            SUBJECT_ID,
            &client::SubjectEdit::new("Ports and adapters", "the body of it")
                .tags(&["rust"])
                .minor(),
        )
        .await
        .unwrap();
    assert_eq!(changed.title, "Ports and adapters".to_owned());
    let request = log.last();
    assert!(
        request.starts_with(&format!("PATCH /api/topics/{SUBJECT_ID} ")),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"minor\":true"), "{request}");
}

#[tokio::test]
async fn a_remark_is_changed_with_a_patch_of_its_own() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .edit_comment("token", SUBJECT_ID, REMARK_ID, "a remark on it")
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with(&format!(
            "PATCH /api/topics/{SUBJECT_ID}/comments/{REMARK_ID} "
        )),
        "{request}"
    );
    assert!(request.contains("\"body\":\"a remark on it\""), "{request}");
}

#[tokio::test]
async fn taking_a_subject_away_is_a_post_that_carries_the_reason() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .delete_topic("token", SUBJECT_ID, &client::Removal::new("off topic"))
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with(&format!("POST /api/topics/{SUBJECT_ID}/delete ")),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"reason\":\"off topic\""), "{request}");
}

#[tokio::test]
async fn taking_a_remark_away_carries_the_reason_and_the_penalty() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    let taken = client
        .delete_comment(
            "token",
            SUBJECT_ID,
            REMARK_ID,
            &client::Removal::new("rude").penalty(2),
        )
        .await
        .unwrap();
    assert_eq!(taken.id, REMARK_ID.to_owned());
    let request = log.last();
    assert!(
        request.starts_with(&format!(
            "POST /api/topics/{SUBJECT_ID}/comments/{REMARK_ID}/delete "
        )),
        "{request}"
    );
    assert!(request.contains("\"penalty\":2"), "{request}");
}

#[tokio::test]
async fn bringing_a_subject_back_is_a_post_of_its_own() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client.restore_topic("token", SUBJECT_ID).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with(&format!("POST /api/topics/{SUBJECT_ID}/restore ")),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn bringing_a_remark_back_is_a_post_of_its_own() {
    let (base, log) = board(answer("200 OK", REMARK)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .restore_comment("token", SUBJECT_ID, REMARK_ID)
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with(&format!(
            "POST /api/topics/{SUBJECT_ID}/comments/{REMARK_ID}/restore "
        )),
        "{request}"
    );
}

#[tokio::test]
async fn the_versions_of_a_subject_are_read_from_its_history() {
    let (base, log) = board(answer("200 OK", HISTORY)).await;
    let client = ApiClient::new(&base).unwrap();
    let versions = client.topic_history(SUBJECT_ID).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].id, VERSION_ID.to_owned());
    assert_eq!(versions[0].editor, "alice".to_owned());
    assert_eq!(
        versions[0].title.as_deref(),
        Some("Ports and adapters".to_owned()).as_deref()
    );
    assert_eq!(versions[0].body, "the body of it".to_owned());
    assert_eq!(versions[0].written_at, "2024-06-07T10:11:12Z".to_owned());
    let request = log.last();
    assert!(
        request.starts_with(&format!("GET /api/topics/{SUBJECT_ID}/history ")),
        "{request}"
    );
}

#[tokio::test]
async fn the_versions_of_a_remark_are_read_from_its_own_history() {
    let (base, log) = board(answer("200 OK", HISTORY)).await;
    let client = ApiClient::new(&base).unwrap();
    client.comment_history(SUBJECT_ID, REMARK_ID).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with(&format!(
            "GET /api/topics/{SUBJECT_ID}/comments/{REMARK_ID}/history "
        )),
        "{request}"
    );
}

#[tokio::test]
async fn what_a_version_changed_is_read_with_the_version() {
    let (base, log) = board(answer("200 OK", CHANGES)).await;
    let client = ApiClient::new(&base).unwrap();
    let changes = client
        .topic_difference(SUBJECT_ID, VERSION_ID)
        .await
        .unwrap();
    let kinds: Vec<&str> = changes.iter().map(|change| change.kind.as_str()).collect();
    assert_eq!(kinds, vec!["kept", "removed", "added"]);
    assert_eq!(changes[1].line, "a line taken away".to_owned());
    let request = log.last();
    assert!(
        request.starts_with(&format!(
            "GET /api/topics/{SUBJECT_ID}/history/{VERSION_ID} "
        )),
        "{request}"
    );
}

#[tokio::test]
async fn a_subject_that_was_not_written_by_the_account_is_refused() {
    let (base, _) = board(answer("403 Forbidden", "{\"error\":\"not allowed\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    match client
        .edit_topic(
            "token",
            SUBJECT_ID,
            &client::SubjectEdit::new("Ports", "body"),
        )
        .await
    {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 403);
            assert_eq!(reason, "not allowed");
        }
        other => panic!("a refusal must keep its reason: {other:?}"),
    }
}
