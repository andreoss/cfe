use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const TAG: &str = concat!(
    "{\"slug\":\"rust\",\"description\":\"The rust language\",",
    "\"means\":\"systems\",\"following\":false}"
);

const FOLLOWED: &str = concat!(
    "{\"slug\":\"rust\",\"description\":\"The rust language\",",
    "\"means\":null,\"following\":true}"
);

const SUBJECTS: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"title\":\"Ports and adapters\",",
    "\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",",
    "\"tags\":[\"rust\",\"adapters\"],\"sticky\":false,\"resolved\":false,",
    "\"deleted\":false,\"pending\":false,\"draft\":false,\"postscore\":2}],",
    "\"page\":{\"number\":2,\"size\":20,\"total\":21,\"total_pages\":2,",
    "\"has_next\":false,\"has_previous\":true}}"
);

const NO_SUBJECTS: &str = concat!(
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

async fn board(tag: &'static str, subjects: &'static str) -> (String, Log) {
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
                sink.lock()
                    .unwrap()
                    .push(format!("{method} {}", target.clone()));
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (answer, body) = match path.as_str() {
                    "/api/me" => ("200 OK", USER),
                    "/api/sections" => ("200 OK", "[]"),
                    other if other.starts_with("/api/tags/") && other.ends_with("/topics") => {
                        ("200 OK", subjects)
                    }
                    other if other.starts_with("/api/tags/") => ("200 OK", tag),
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

async fn token_of(base: &str, address: &str) -> String {
    let response = http()
        .get(format!("{base}{address}"))
        .header("cookie", "session=token")
        .send()
        .await
        .unwrap();
    let token = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(';').next())
        .and_then(|pair| pair.strip_prefix("token="))
        .expect("a page answers with a token")
        .to_owned();
    response.text().await.unwrap();
    token
}

#[tokio::test]
async fn a_tag_page_asks_the_board_for_the_tag_and_for_its_subjects() {
    let (board_url, log) = board(TAG, SUBJECTS).await;
    let base = front(&board_url).await;
    page(&base, "/tags/rust").await;
    assert_eq!(
        log.calls(),
        vec![
            "GET /api/tags/rust".to_owned(),
            "GET /api/tags/rust/topics?page=1".to_owned(),
        ]
    );
}

#[tokio::test]
async fn a_tag_page_carries_the_tag_the_words_about_it_and_its_subjects() {
    let (board_url, _) = board(TAG, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/rust").await;
    assert!(page.contains("<h2>rust</h2>"), "{page}");
    assert!(
        page.contains("<p class=\"tag-words\">The rust language</p>"),
        "{page}"
    );
    assert!(page.contains("<h3>21 subjects</h3>"), "{page}");
    assert!(
        page.contains(
            "<a href=\"/topics/11111111-1111-1111-1111-111111111111\">Ports and adapters</a>"
        ),
        "{page}"
    );
    assert!(page.contains(">alice<"), "{page}");
    assert!(page.contains("<time datetime=\"2024-06-07T10:11:12Z\">2024-06-07 10:11</time>"));
    assert!(
        page.contains("<a href=\"/tags/adapters\">adapters</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn the_subjects_of_a_tag_are_read_a_page_at_a_time() {
    let (board_url, log) = board(TAG, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/rust?page=2").await;
    assert!(
        log.calls()
            .contains(&"GET /api/tags/rust/topics?page=2".to_owned()),
        "{:?}",
        log.calls()
    );
    assert!(page.contains("Page 2 of 2"), "{page}");
    assert!(
        page.contains("<a rel=\"prev\" href=\"/tags/rust?page=1\">Previous page</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_tag_that_means_another_says_so_and_points_at_it() {
    let (board_url, _) = board(TAG, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/rust").await;
    assert!(
        page.contains("Means <a href=\"/tags/systems\">systems</a>."),
        "{page}"
    );
}

#[tokio::test]
async fn a_tag_nobody_used_says_so_and_carries_no_subjects() {
    let (board_url, _) = board(TAG, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/rust").await;
    assert!(page.contains("<h3>0 subjects</h3>"), "{page}");
    assert!(page.contains("No subject carries this tag yet."), "{page}");
}

#[tokio::test]
async fn the_words_about_a_tag_stay_words_and_carry_no_scripting() {
    let (board_url, _) = board(
        "{\"slug\":\"a%3Cb%3E\",\"description\":\"<script>alert(1)</script>\",\"means\":null,\"following\":false}",
        NO_SUBJECTS,
    )
    .await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/a%3Cb%3E").await;
    assert!(
        front::html::scripting_free(&page),
        "not scripting free: {page}"
    );
    assert!(page.contains("&lt;script&gt;"), "{page}");
}

#[tokio::test]
async fn following_a_tag_is_offered_to_an_account_and_leaving_to_a_follower() {
    let (board_url, _) = board(TAG, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/tags/rust").await;
    assert!(page.contains("action=\"/tags/rust/follow\""), "{page}");
    assert!(page.contains("Follow this tag"), "{page}");
    assert!(!page.contains("/tags/rust/unfollow"), "{page}");

    let (board_url, _) = board(FOLLOWED, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/tags/rust").await;
    assert!(page.contains("action=\"/tags/rust/unfollow\""), "{page}");
    assert!(page.contains("Leave this tag"), "{page}");
}

#[tokio::test]
async fn nobody_is_offered_the_way_to_follow_a_tag() {
    let (board_url, _) = board(TAG, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/tags/rust").await;
    assert!(!page.contains("/tags/rust/follow"), "{page}");
}

#[tokio::test]
async fn following_a_tag_asks_the_board_and_comes_back_to_the_tag() {
    let (board_url, log) = board(TAG, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/tags/rust").await;
    let response = http()
        .post(format!("{base}/tags/rust/follow"))
        .header("cookie", format!("session=token; token={token}"))
        .form(&[("token", token.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/tags/rust")
    );
    assert!(
        log.calls()
            .contains(&"POST /api/tags/rust/follow".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn saying_what_a_tag_means_asks_the_board_and_comes_back_to_the_tag() {
    let (board_url, log) = board(TAG, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/tags/rust").await;
    let response = http()
        .post(format!("{base}/tags/rust/describe"))
        .header("cookie", format!("session=token; token={token}"))
        .form(&[
            ("token", token.as_str()),
            ("description", "Words about it"),
            ("means", "systems"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/tags/rust")
    );
    assert!(
        log.calls().contains(&"PATCH /api/tags/rust".to_owned()),
        "{:?}",
        log.calls()
    );
    assert!(
        log.calls()
            .contains(&"POST /api/tags/rust/means".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_form_without_a_token_is_refused() {
    let (board_url, log) = board(TAG, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let response = http()
        .post(format!("{base}/tags/rust/follow"))
        .header("cookie", "session=token")
        .form(&[("token", "stolen")])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("The form has expired")
    );
    assert!(
        !log.calls()
            .contains(&"POST /api/tags/rust/follow".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_board_that_refuses_a_tag_gives_a_page_not_a_crash() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(_) => return,
            };
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 8192];
                let _ = socket.read(&mut buffer).await;
                let body = "{\"error\":\"broken\"}";
                let response = format!(
                    "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    let base = front(&format!("http://{address}")).await;
    let response = http()
        .get(format!("{base}/tags/rust"))
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
