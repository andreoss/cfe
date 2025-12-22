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

fn raw(status: &'static str, kind: &'static str, body: &'static str) -> &'static str {
    let mut response = format!("HTTP/1.1 {status}\r\n");
    response.push_str(&format!("content-type: {kind}\r\n"));
    response.push_str(&format!("content-length: {}\r\n", body.len()));
    response.push_str("connection: close\r\n\r\n");
    Box::leak(Box::new(response + body))
}

const SUBJECT: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"group_slug\":\"team\",",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"tags\":[\"rust\"],\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"postscore\":0,",
    "\"pending\":false,\"draft\":false,\"sticky\":false,",
    "\"off_front\":false,\"resolved\":false,\"minor\":false,",
    "\"open_reports\":0}"
);

const IMAGES: &str = concat!(
    "[{\"id\":\"55555555-5555-5555-5555-555555555555\",",
    "\"content_type\":\"image/png\",\"uploaded_by\":\"alice\"}]"
);

const IMAGE: &str = concat!(
    "{\"id\":\"55555555-5555-5555-5555-555555555555\",",
    "\"content_type\":\"image/png\",\"uploaded_by\":\"alice\"}"
);

const GROUPS: &str = concat!(
    "[{\"id\":\"66666666-6666-6666-6666-666666666666\",",
    "\"section_slug\":\"general\",\"name\":\"Team\",\"slug\":\"team\"}]"
);

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const IMAGE_ID: &str = "55555555-5555-5555-5555-555555555555";

#[tokio::test]
async fn a_subject_is_read_with_the_session_so_a_pending_one_is_shown() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    let subject = client.subject(SUBJECT_ID, Some("token")).await.unwrap();
    assert_eq!(subject.id, SUBJECT_ID.to_owned());
    let request = log.last();
    assert!(
        request.starts_with("GET /api/topics/11111111-1111-1111-1111-111111111111 "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_subject_and_its_remarks_are_read_without_one_as_well() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.subject(SUBJECT_ID, None).await.is_ok());
    assert!(!log.last().contains("cookie:"), "{}", log.last());
}

#[tokio::test]
async fn a_draft_is_published_with_the_session() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    let subject = client.publish("token", SUBJECT_ID).await.unwrap();
    assert_eq!(subject.title, "Ports and adapters".to_owned());
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/publish "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_subject_is_committed_and_committed_no_more_with_the_session() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client.commit("token", SUBJECT_ID).await.unwrap();
    assert!(
        log.last()
            .starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/commit "),
        "{}",
        log.last()
    );
    client.uncommit("token", SUBJECT_ID).await.unwrap();
    assert!(
        log.last()
            .starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/uncommit "),
        "{}",
        log.last()
    );
    assert!(
        log.last().contains("cookie: session=token"),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn a_subject_is_pinned_and_unpinned_with_the_session() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(
        !client
            .sticky("token", SUBJECT_ID, true)
            .await
            .unwrap()
            .sticky
    );
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/sticky "),
        "{request}"
    );
    assert!(request.contains("\"sticky\":true"), "{request}");
    client.sticky("token", SUBJECT_ID, false).await.unwrap();
    assert!(log.last().contains("\"sticky\":false"), "{}", log.last());
}

#[tokio::test]
async fn a_subject_is_kept_off_the_front_page_and_put_back() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client.off_front("token", SUBJECT_ID, true).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/off-front "),
        "{request}"
    );
    assert!(request.contains("\"off_front\":true"), "{request}");
    client.off_front("token", SUBJECT_ID, false).await.unwrap();
    assert!(log.last().contains("\"off_front\":false"), "{}", log.last());
}

#[tokio::test]
async fn a_subject_is_marked_resolved_and_opened_again() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client.resolved("token", SUBJECT_ID, true).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/resolved "),
        "{request}"
    );
    assert!(request.contains("\"resolved\":true"), "{request}");
    client.resolved("token", SUBJECT_ID, false).await.unwrap();
    assert!(log.last().contains("\"resolved\":false"), "{}", log.last());
}

#[tokio::test]
async fn the_score_of_a_subject_is_set_with_the_session() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client.postscore("token", SUBJECT_ID, -3).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/postscore "),
        "{request}"
    );
    assert!(request.contains("\"postscore\":-3"), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_subject_is_moved_to_another_of_its_section_groups() {
    let (base, log) = board(answer("200 OK", SUBJECT)).await;
    let client = ApiClient::new(&base).unwrap();
    client.move_to("token", SUBJECT_ID, "team").await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/move "),
        "{request}"
    );
    assert!(request.contains("\"group\":\"team\""), "{request}");
}

#[tokio::test]
async fn the_groups_of_a_section_are_read_from_the_api() {
    let (base, log) = board(answer("200 OK", GROUPS)).await;
    let client = ApiClient::new(&base).unwrap();
    let groups = client.groups("general").await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].slug, "team".to_owned());
    assert_eq!(groups[0].name, "Team".to_owned());
    assert!(
        log.last().starts_with("GET /api/sections/general/groups "),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn the_pictures_of_a_subject_are_read_from_the_api() {
    let (base, log) = board(answer("200 OK", IMAGES)).await;
    let client = ApiClient::new(&base).unwrap();
    let images = client.images(SUBJECT_ID).await.unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].id, IMAGE_ID.to_owned());
    assert_eq!(images[0].content_type, "image/png".to_owned());
    assert_eq!(images[0].uploaded_by, "alice".to_owned());
    assert!(
        log.last()
            .starts_with("GET /api/topics/11111111-1111-1111-1111-111111111111/images "),
        "{}",
        log.last()
    );
}

#[tokio::test]
async fn a_picture_is_read_as_its_bytes_and_its_kind() {
    let (base, _) = board(raw("200 OK", "image/png", "PNGDATA")).await;
    let client = ApiClient::new(&base).unwrap();
    let picture = client.image(SUBJECT_ID, IMAGE_ID).await.unwrap().unwrap();
    assert_eq!(picture.content_type(), "image/png");
    assert_eq!(picture.bytes(), b"PNGDATA");
}

#[tokio::test]
async fn a_picture_nobody_knows_has_no_bytes() {
    let (base, _) = board(raw("404 Not Found", "image/png", "x")).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.image(SUBJECT_ID, IMAGE_ID).await.unwrap().is_none());
}

#[tokio::test]
async fn a_picture_is_put_up_with_the_session() {
    let (base, log) = board(answer("201 Created", IMAGE)).await;
    let client = ApiClient::new(&base).unwrap();
    let image = client
        .attach_image("token", SUBJECT_ID, "aGVsbG8=")
        .await
        .unwrap();
    assert_eq!(image.id, IMAGE_ID.to_owned());
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/images "),
        "{request}"
    );
    assert!(request.contains("\"data\":\"aGVsbG8=\""), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_picture_is_taken_down_with_the_session() {
    let (base, log) = board(answer("204 No Content", "")).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .remove_image("token", SUBJECT_ID, IMAGE_ID)
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with(
            "DELETE /api/topics/11111111-1111-1111-1111-111111111111/images/55555555-5555-5555-5555-555555555555 "
        ),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_refusal_from_the_board_keeps_its_reason() {
    let base = board(answer(
        "403 Forbidden",
        "{\"error\":\"moderator role required\"}",
    ))
    .await
    .0;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.sticky("token", SUBJECT_ID, true).await.unwrap_err(),
        ClientError::Rejected {
            status: 403,
            reason: "moderator role required".to_owned(),
        }
    );
    assert_eq!(
        client.publish("token", SUBJECT_ID).await.unwrap_err(),
        ClientError::Rejected {
            status: 403,
            reason: "moderator role required".to_owned(),
        }
    );
    assert_eq!(
        client.postscore("token", SUBJECT_ID, 1).await.unwrap_err(),
        ClientError::Rejected {
            status: 403,
            reason: "moderator role required".to_owned(),
        }
    );
}
