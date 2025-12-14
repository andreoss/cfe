use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECTIONS: &str = "[{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"anyone\",\"may_post\":true},{\"slug\":\"news\",\"title\":\"News\",\"topics_score\":\"moderators\",\"may_post\":false}]";

const TOPICS: &str = "{\"items\":[{\"id\":\"11111111-1111-1111-1111-111111111111\",\"section_slug\":\"general\",\"title\":\"First subject\",\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",\"tags\":[\"rust\"],\"sticky\":true,\"resolved\":false,\"deleted\":false,\"pending\":false,\"draft\":false,\"postscore\":2},{\"id\":\"22222222-2222-2222-2222-222222222222\",\"section_slug\":\"general\",\"title\":\"Second subject\",\"author_username\":\"bob\",\"created_at\":\"2024-06-08T09:08:07Z\",\"tags\":[],\"sticky\":false,\"resolved\":true,\"deleted\":false,\"pending\":false,\"draft\":false,\"postscore\":0}],\"page\":{\"number\":1,\"size\":25,\"total\":30,\"total_pages\":2,\"has_next\":true,\"has_previous\":false}}";

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn targets(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(topics_status: &'static str) -> (String, Log) {
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
                let (status, body) = match target.split('?').next().unwrap_or("") {
                    "/api/sections" => ("200 OK", SECTIONS),
                    path if path.starts_with("/api/sections/") && path.ends_with("/topics") => {
                        (topics_status, TOPICS)
                    }
                    _ => ("404 Not Found", "{\"error\":\"not found\"}"),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
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

#[tokio::test]
async fn a_section_page_lists_the_subjects_the_board_answers_with() {
    let (board_url, _) = board("200 OK").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/sections/general"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>General Talk</h2>"));
    assert!(page.contains("href=\"/topics/11111111-1111-1111-1111-111111111111\""));
    assert!(page.contains("First subject"));
    assert!(page.contains("Second subject"));
    assert!(page.contains(">alice<"));
    assert!(page.contains(">2024-06-07 10:11<"));
    assert!(page.contains("href=\"/tags/rust\""));
    assert!(page.contains("(pinned)"));
    assert!(page.contains("(resolved)"));
    assert!(page.contains("<caption>30 subjects</caption>"));
}

#[tokio::test]
async fn the_page_in_the_address_is_asked_of_the_board() {
    let (board_url, log) = board("200 OK").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/sections/general?page=2"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<p>Page 1 of 2</p>"));
    assert!(
        log.targets()
            .contains(&"/api/sections/general/topics?page=2".to_owned())
    );
}

#[tokio::test]
async fn a_page_number_nobody_can_read_means_the_first_page() {
    let (board_url, log) = board("200 OK").await;
    let base = front(&board_url).await;
    for asked in ["?page=abc", "?page=0", "?page=-3", "?page=99999", "?page="] {
        http()
            .get(format!("{base}/sections/general{asked}"))
            .send()
            .await
            .unwrap();
    }
    let asked: Vec<String> = log
        .targets()
        .into_iter()
        .filter(|target| target.contains("/topics"))
        .collect();
    assert_eq!(
        asked,
        vec![
            "/api/sections/general/topics?page=1",
            "/api/sections/general/topics?page=1",
            "/api/sections/general/topics?page=1",
            "/api/sections/general/topics?page=1",
            "/api/sections/general/topics?page=1",
        ]
    );
}

#[tokio::test]
async fn a_section_page_offers_the_next_page() {
    let (board_url, _) = board("200 OK").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/sections/general"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("href=\"/sections/general?page=2\""));
    assert!(page.contains("rel=\"next\""));
    assert!(!page.contains("rel=\"prev\""));
}

#[tokio::test]
async fn a_section_page_carries_no_scripting_and_keeps_its_own_address() {
    let (board_url, _) = board("200 OK").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/sections/general?page=2"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        front::html::scripting_free(&page),
        "not scripting free: {page}"
    );
    assert!(page.contains("value=\"/sections/general?page=2\""));
}

#[tokio::test]
async fn a_section_that_is_not_there_is_a_page_and_not_a_crash() {
    let (board_url, _) = board("200 OK").await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/sections/nothing"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert!(response.text().await.unwrap().contains("No such section"));
}

#[tokio::test]
async fn a_board_that_refuses_the_subjects_gives_a_page_not_a_crash() {
    let (board_url, _) = board("500 Internal Server Error").await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/sections/general"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 503);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("The board is not answering")
    );
}
