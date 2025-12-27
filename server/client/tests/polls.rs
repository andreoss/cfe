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

const POLL: &str = concat!(
    "{\"id\":\"77777777-7777-7777-7777-777777777777\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"question\":\"Which way?\",\"options\":[",
    "{\"id\":\"88888888-8888-8888-8888-888888888888\",\"text\":\"Ports\",\"votes\":2},",
    "{\"id\":\"99999999-9999-9999-9999-999999999999\",\"text\":\"Adapters\",\"votes\":1}],",
    "\"mine\":\"88888888-8888-8888-8888-888888888888\",\"total_votes\":3}"
);

const REACTIONS: &str = concat!(
    "{\"counts\":[{\"kind\":\"like\",\"count\":2},",
    "{\"kind\":\"thanks\",\"count\":1}],\"mine\":\"like\"}"
);

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const REMARK_ID: &str = "22222222-2222-2222-2222-222222222222";
const OPTION_ID: &str = "88888888-8888-8888-8888-888888888888";

#[tokio::test]
async fn the_poll_of_a_subject_is_read_with_the_session() {
    let (base, log) = board(answer("200 OK", POLL)).await;
    let client = ApiClient::new(&base).unwrap();
    let poll = client
        .poll(Some("token"), SUBJECT_ID)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(poll.question, "Which way?".to_owned());
    assert_eq!(poll.options.len(), 2);
    assert_eq!(poll.options[0].votes, 2);
    assert_eq!(poll.total_votes, 3);
    assert_eq!(poll.mine, Some(OPTION_ID.to_owned()));
    let request = log.last();
    assert!(
        request.starts_with("GET /api/topics/11111111-1111-1111-1111-111111111111/poll "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_poll_is_read_without_one_as_well() {
    let (base, log) = board(answer("200 OK", POLL)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.poll(None, SUBJECT_ID).await.unwrap().is_some());
    assert!(!log.last().contains("cookie:"), "{}", log.last());
}

#[tokio::test]
async fn a_subject_without_a_poll_has_none() {
    let (base, _) = board(answer("404 Not Found", "{\"error\":\"poll not found\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.poll(None, SUBJECT_ID).await.unwrap().is_none());
}

#[tokio::test]
async fn a_poll_is_refused_and_the_reason_is_kept() {
    let (base, _) = board(answer(
        "409 Conflict",
        "{\"error\":\"poll already exists\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let error = client
        .create_poll("token", SUBJECT_ID, "Which way?", &["Ports", "Adapters"])
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(409));
    assert!(
        matches!(error, ClientError::Rejected { reason, .. } if reason == "poll already exists")
    );
}

#[tokio::test]
async fn a_poll_is_put_up_with_the_session() {
    let (base, log) = board(answer("200 OK", POLL)).await;
    let client = ApiClient::new(&base).unwrap();
    let poll = client
        .create_poll("token", SUBJECT_ID, "Which way?", &["Ports", "Adapters"])
        .await
        .unwrap();
    assert_eq!(poll.id, "77777777-7777-7777-7777-777777777777".to_owned());
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/poll "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"question\":\"Which way?\""), "{request}");
    assert!(
        request.contains("\"options\":[\"Ports\",\"Adapters\"]"),
        "{request}"
    );
}

#[tokio::test]
async fn a_vote_is_cast_on_one_of_the_options() {
    let (base, log) = board(answer("200 OK", POLL)).await;
    let client = ApiClient::new(&base).unwrap();
    let poll = client.vote("token", SUBJECT_ID, OPTION_ID).await.unwrap();
    assert_eq!(poll.total_votes, 3);
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/poll/vote "),
        "{request}"
    );
    assert!(
        request.contains(&format!("\"option_id\":\"{OPTION_ID}\"")),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_reactions_of_a_subject_are_read_with_the_session() {
    let (base, log) = board(answer("200 OK", REACTIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    let reactions = client
        .topic_reactions(Some("token"), SUBJECT_ID)
        .await
        .unwrap();
    assert_eq!(reactions.counts.len(), 2);
    assert_eq!(reactions.counts[0].kind, "like".to_owned());
    assert_eq!(reactions.counts[0].count, 2);
    assert_eq!(reactions.mine, Some("like".to_owned()));
    let request = log.last();
    assert!(
        request.starts_with("GET /api/topics/11111111-1111-1111-1111-111111111111/reactions "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_reactions_of_a_remark_are_read() {
    let (base, log) = board(answer("200 OK", REACTIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(
        client
            .comment_reactions(None, SUBJECT_ID, REMARK_ID)
            .await
            .is_ok()
    );
    let request = log.last();
    assert!(
        request.starts_with(
            "GET /api/topics/11111111-1111-1111-1111-111111111111/comments/22222222-2222-2222-2222-222222222222/reactions "
        ),
        "{request}"
    );
}

#[tokio::test]
async fn a_subject_is_reacted_to_with_the_session() {
    let (base, log) = board(answer("200 OK", REACTIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    let reactions = client
        .react_to_topic("token", SUBJECT_ID, "like")
        .await
        .unwrap();
    assert_eq!(reactions.mine, Some("like".to_owned()));
    let request = log.last();
    assert!(
        request.starts_with("POST /api/topics/11111111-1111-1111-1111-111111111111/reactions "),
        "{request}"
    );
    assert!(request.contains("{\"kind\":\"like\"}"), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_remark_is_reacted_to_with_the_session() {
    let (base, log) = board(answer("200 OK", REACTIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(
        client
            .react_to_comment("token", SUBJECT_ID, REMARK_ID, "thanks")
            .await
            .is_ok()
    );
    let request = log.last();
    assert!(
        request.starts_with(
            "POST /api/topics/11111111-1111-1111-1111-111111111111/comments/22222222-2222-2222-2222-222222222222/reactions "
        ),
        "{request}"
    );
    assert!(request.contains("{\"kind\":\"thanks\"}"), "{request}");
}

#[tokio::test]
async fn a_reaction_on_a_subject_is_taken_away() {
    let (base, log) = board(answer("200 OK", REACTIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(
        client
            .clear_topic_reaction("token", SUBJECT_ID)
            .await
            .is_ok()
    );
    let request = log.last();
    assert!(
        request.starts_with("DELETE /api/topics/11111111-1111-1111-1111-111111111111/reactions "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_reaction_on_a_remark_is_taken_away() {
    let (base, log) = board(answer("200 OK", REACTIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(
        client
            .clear_comment_reaction("token", SUBJECT_ID, REMARK_ID)
            .await
            .is_ok()
    );
    let request = log.last();
    assert!(
        request.starts_with(
            "DELETE /api/topics/11111111-1111-1111-1111-111111111111/comments/22222222-2222-2222-2222-222222222222/reactions "
        ),
        "{request}"
    );
}
