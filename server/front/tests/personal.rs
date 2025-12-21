use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const KEPT: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"title\":\"Ports and adapters\",",
    "\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",",
    "\"tags\":[\"rust\",\"adapters\"],\"sticky\":false,\"resolved\":false,",
    "\"deleted\":false,\"pending\":false,\"draft\":false,\"postscore\":2}],",
    "\"page\":{\"number\":2,\"size\":20,\"total\":21,\"total_pages\":2,",
    "\"has_next\":false,\"has_previous\":true}}"
);

const NONE_KEPT: &str = concat!(
    "{\"items\":[],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":0,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const USER: &str = "{\"id\":\"9\",\"username\":\"alice\",\"role\":\"user\"}";

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(kept: &'static str) -> (String, Log) {
    board_with(kept, "200 OK").await
}

async fn board_with(kept: &'static str, answer: &'static str) -> (String, Log) {
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
                let method = request
                    .split_whitespace()
                    .next()
                    .unwrap_or("GET")
                    .to_owned();
                let target = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                sink.lock().unwrap().push(format!("{method} {target}"));
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (answer, body) = match path.as_str() {
                    "/api/me" => ("200 OK", USER),
                    "/api/sections" => ("200 OK", "[]"),
                    "/api/bookmarks" | "/api/watched" => (answer, kept),
                    _ => ("404 Not Found", "{\"error\":\"not found\"}"),
                };
                let response = format!(
                    "HTTP/1.1 {answer}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
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

async fn signed_page(base: &str, address: &str) -> String {
    http()
        .get(format!("{base}{address}"))
        .header("cookie", "session=token")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

#[tokio::test]
async fn a_bookmarks_page_asks_the_board_with_the_session_it_holds() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    signed_page(&base, "/bookmarks").await;
    assert_eq!(
        log.calls(),
        vec![
            "GET /api/me".to_owned(),
            "GET /api/bookmarks?page=1".to_owned()
        ]
    );
}

#[tokio::test]
async fn the_bookmarks_are_read_a_page_at_a_time() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/bookmarks?page=2").await;
    assert!(
        log.calls()
            .contains(&"GET /api/bookmarks?page=2".to_owned()),
        "{:?}",
        log.calls()
    );
    assert!(page.contains("Page 2 of 2"), "{page}");
    assert!(
        page.contains("<a rel=\"prev\" href=\"/bookmarks?page=1\">Previous page</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_bookmarks_page_carries_the_subjects_that_were_kept() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/bookmarks").await;
    assert!(page.contains("<h2>Bookmarks</h2>"), "{page}");
    assert!(page.contains("<h3>21 subjects</h3>"), "{page}");
    assert!(
        page.contains(
            "<a href=\"/topics/11111111-1111-1111-1111-111111111111\">Ports and adapters</a>"
        ),
        "{page}"
    );
    assert!(page.contains(">alice<"), "{page}");
    assert!(
        page.contains("<time datetime=\"2024-06-07T10:11:12Z\">2024-06-07 10:11</time>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_bookmarks_list_that_holds_nothing_says_so() {
    let (board_url, _) = board(NONE_KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/bookmarks").await;
    assert!(page.contains("<h3>0 subjects</h3>"), "{page}");
    assert!(page.contains("Nothing has been kept yet."), "{page}");
}

#[tokio::test]
async fn a_watched_page_asks_the_board_with_the_session_it_holds() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    signed_page(&base, "/watched").await;
    assert!(
        log.calls().contains(&"GET /api/watched?page=1".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_watched_page_carries_the_subjects_that_are_watched() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/watched").await;
    assert!(page.contains("<h2>Watched</h2>"), "{page}");
    assert!(page.contains("<h3>21 subjects</h3>"), "{page}");
    assert!(
        page.contains(
            "<a href=\"/topics/11111111-1111-1111-1111-111111111111\">Ports and adapters</a>"
        ),
        "{page}"
    );
}

#[tokio::test]
async fn a_watched_list_that_holds_nothing_says_so() {
    let (board_url, _) = board(NONE_KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/watched").await;
    assert!(page.contains("Nothing is watched yet."), "{page}");
}

#[tokio::test]
async fn a_watched_list_is_read_a_page_at_a_time() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/watched?page=2").await;
    assert!(
        log.calls().contains(&"GET /api/watched?page=2".to_owned()),
        "{:?}",
        log.calls()
    );
    assert!(
        page.contains("<a rel=\"prev\" href=\"/watched?page=1\">Previous page</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_reader_with_no_session_is_sent_to_sign_in_first() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    for address in ["/bookmarks", "/watched"] {
        let response = http().get(format!("{base}{address}")).send().await.unwrap();
        assert_eq!(response.status(), 303, "{address}");
        assert_eq!(
            response
                .headers()
                .get("location")
                .unwrap()
                .to_str()
                .unwrap(),
            format!("/sign-in?return_to={address}"),
            "{address}"
        );
    }
    assert!(
        !log.calls().iter().any(|call| call.contains("bookmarks")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_pages_of_an_account_are_offered_to_the_account_that_holds_them() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let held = signed_page(&base, "/bookmarks").await;
    assert!(
        held.contains("<a href=\"/bookmarks\">Bookmarks</a>"),
        "{held}"
    );
    assert!(held.contains("<a href=\"/watched\">Watched</a>"), "{held}");
    let (other_url, _) = board(KEPT).await;
    let other = front(&other_url).await;
    let anonymous = http()
        .get(format!("{other}/archive"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        !anonymous.contains("<a href=\"/bookmarks\">Bookmarks</a>"),
        "{anonymous}"
    );
}

#[tokio::test]
async fn a_board_that_refuses_what_was_kept_gives_a_page_and_not_a_crash() {
    let (board_url, _) = board_with("{\"error\":\"broken\"}", "500 Internal Server Error").await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/bookmarks"))
        .header("cookie", "session=token")
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
