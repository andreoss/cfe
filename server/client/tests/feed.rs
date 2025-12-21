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

const FEED: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
    "<feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
    "  <title>general</title>\n",
    "</feed>\n"
);

const ATOM: &[&str] = &["content-type: application/atom+xml; charset=utf-8"];

#[tokio::test]
async fn the_feed_of_a_section_is_asked_for_by_its_address() {
    let (base, log) = board(answer("200 OK", ATOM, FEED)).await;
    let client = ApiClient::new(&base).unwrap();
    client.section_feed("general").await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("GET /api/sections/general/feed "),
        "{request}"
    );
}

#[tokio::test]
async fn the_feed_of_a_tag_is_asked_for_by_its_address() {
    let (base, log) = board(answer("200 OK", ATOM, FEED)).await;
    let client = ApiClient::new(&base).unwrap();
    client.tag_feed("rust").await.unwrap();
    let request = log.last();
    assert!(request.starts_with("GET /api/tags/rust/feed "), "{request}");
}

#[tokio::test]
async fn a_feed_keeps_the_kind_it_was_given_and_its_words() {
    let (base, _) = board(answer("200 OK", ATOM, FEED)).await;
    let client = ApiClient::new(&base).unwrap();
    let feed = client.section_feed("general").await.unwrap().unwrap();
    assert_eq!(feed.content_type(), "application/atom+xml; charset=utf-8");
    assert!(feed.body().contains("<title>general</title>"));
}

#[tokio::test]
async fn a_feed_the_board_does_not_have_answers_with_nothing() {
    let (base, _) = board(answer(
        "404 Not Found",
        &["content-type: application/json"],
        "{\"error\":\"not found\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.tag_feed("rust").await.unwrap().is_none());
}

#[tokio::test]
async fn a_feed_the_board_refuses_keeps_its_reason() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        &["content-type: application/json"],
        "{\"error\":\"invalid tag\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client.tag_feed("rust!").await {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 422);
            assert_eq!(reason, "invalid tag");
        }
        other => panic!("a refused feed must keep its reason: {other:?}"),
    }
}
