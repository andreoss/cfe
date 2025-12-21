use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const FEED: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
    "<feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
    "  <title>general</title>\n",
    "</feed>\n"
);

const SECTION: &str = concat!(
    "{\"slug\":\"general\",\"title\":\"General Talk\",",
    "\"topics_score\":\"anyone\",\"may_post\":true}"
);

const SUBJECTS: &str = concat!(
    "{\"items\":[],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":0,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const TAG: &str = "{\"slug\":\"rust\",\"description\":null,\"means\":null,\"following\":false}";

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(with_feed: bool) -> (String, Log) {
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
                let target = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                sink.lock().unwrap().push(target.clone());
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (answer, kind, body) = match path.as_str() {
                    "/api/sections" => ("200 OK", "application/json", format!("[{SECTION}]")),
                    "/api/sections/general/topics" => {
                        ("200 OK", "application/json", SUBJECTS.to_owned())
                    }
                    "/api/tags/rust" => ("200 OK", "application/json", TAG.to_owned()),
                    "/api/tags/rust/topics" => ("200 OK", "application/json", SUBJECTS.to_owned()),
                    other if other.ends_with("/feed") && with_feed => (
                        "200 OK",
                        "application/atom+xml; charset=utf-8",
                        FEED.to_owned(),
                    ),
                    _ => (
                        "404 Not Found",
                        "application/json",
                        "{\"error\":\"not found\"}".to_owned(),
                    ),
                };
                let response = format!(
                    "HTTP/1.1 {answer}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
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
    let mut vars = std::collections::BTreeMap::new();
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

async fn page(base: &str, address: &str) -> String {
    http()
        .get(format!("{base}{address}"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

#[tokio::test]
async fn a_section_offers_the_feed_of_its_subjects() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let page = page(&base, "/sections/general").await;
    assert!(
        page.contains("<a href=\"/sections/general/feed\" type=\"application/atom+xml\">"),
        "{page}"
    );
}

#[tokio::test]
async fn a_tag_offers_the_feed_of_its_subjects() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/rust").await;
    assert!(
        page.contains("<a href=\"/tags/rust/feed\" type=\"application/atom+xml\">"),
        "{page}"
    );
}

#[tokio::test]
async fn the_feed_of_a_section_comes_back_as_a_feed() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/sections/general/feed"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/atom+xml; charset=utf-8")
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("<title>general</title>"), "{body}");
    assert!(
        log.calls()
            .contains(&"/api/sections/general/feed".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_feed_of_a_tag_comes_back_as_a_feed() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/tags/rust/feed"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/atom+xml; charset=utf-8")
    );
    assert!(
        log.calls().contains(&"/api/tags/rust/feed".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_board_without_a_feed_answers_with_nothing_here() {
    let (board_url, _) = board(false).await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/tags/rust/feed"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn a_feed_is_not_a_page_and_carries_no_scripting() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let body = http()
        .get(format!("{base}/sections/general/feed"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!body.contains("<!DOCTYPE html>"), "{body}");
    assert!(
        front::html::scripting_free(&body),
        "not scripting free: {body}"
    );
}
