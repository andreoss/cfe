use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MONTHS: &str = concat!(
    "[{\"year\":2024,\"month\":6,\"topics\":21},",
    "{\"year\":2024,\"month\":5,\"topics\":4},",
    "{\"year\":2023,\"month\":12,\"topics\":1}]"
);

const NO_MONTHS: &str = "[]";

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

async fn board(months: &'static str, subjects: &'static str) -> (String, Log) {
    board_with(months, subjects, "200 OK").await
}

async fn board_with(
    months: &'static str,
    subjects: &'static str,
    month_answer: &'static str,
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
                    "/api/archive" => ("200 OK", months),
                    other if other.starts_with("/api/archive/") => (month_answer, subjects),
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

#[tokio::test]
async fn an_archive_page_asks_the_board_for_the_months() {
    let (board_url, log) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    page(&base, "/archive").await;
    assert_eq!(log.calls(), vec!["GET /api/archive".to_owned()]);
}

#[tokio::test]
async fn the_months_are_read_with_the_session_when_one_is_held() {
    let (board_url, log) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    signed_page(&base, "/archive").await;
    assert!(
        log.calls().contains(&"GET /api/archive".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn an_archive_page_carries_every_month_and_how_many_subjects_it_carries() {
    let (board_url, _) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/archive").await;
    assert!(
        page.contains("<a href=\"/archive/2024/6\">June 2024</a>"),
        "{page}"
    );
    assert!(
        page.contains("<a href=\"/archive/2023/12\">December 2023</a>"),
        "{page}"
    );
    assert!(page.contains(">21 subjects<"), "{page}");
    assert!(page.contains(">1 subject<"), "{page}");
}

#[tokio::test]
async fn an_archive_nobody_wrote_in_says_so() {
    let (board_url, _) = board(NO_MONTHS, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/archive").await;
    assert!(page.contains("<h2>Archive</h2>"), "{page}");
    assert!(page.contains("Nothing has been written yet."), "{page}");
}

#[tokio::test]
async fn a_month_page_asks_the_board_for_the_subjects_of_that_month() {
    let (board_url, log) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    page(&base, "/archive/2024/6").await;
    assert_eq!(
        log.calls(),
        vec!["GET /api/archive/2024/6?page=1".to_owned()]
    );
}

#[tokio::test]
async fn the_subjects_of_a_month_are_read_a_page_at_a_time() {
    let (board_url, log) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/archive/2024/6?page=2").await;
    assert!(
        log.calls()
            .contains(&"GET /api/archive/2024/6?page=2".to_owned()),
        "{:?}",
        log.calls()
    );
    assert!(page.contains("Page 2 of 2"), "{page}");
    assert!(
        page.contains("<a rel=\"prev\" href=\"/archive/2024/6?page=1\">Previous page</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_month_page_carries_the_month_and_the_subjects_written_in_it() {
    let (board_url, _) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/archive/2024/6").await;
    assert!(page.contains("<h2>June 2024</h2>"), "{page}");
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
    assert!(
        page.contains("<a href=\"/archive\">The archive</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_month_nobody_wrote_in_says_so() {
    let (board_url, _) = board(MONTHS, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/archive/2024/6").await;
    assert!(
        page.contains("No subject was written in this month."),
        "{page}"
    );
}

#[tokio::test]
async fn a_month_the_board_refuses_gives_a_page_and_not_a_crash() {
    let (board_url, _) = board_with(
        MONTHS,
        "{\"error\":\"no such month\"}",
        "422 Unprocessable Entity",
    )
    .await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/archive/2024/13"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert!(
        response.text().await.unwrap().contains("No such month"),
        "no such month page"
    );
}

#[tokio::test]
async fn a_month_that_is_no_number_is_refused_and_not_guessed() {
    let (board_url, _) = board(MONTHS, NO_SUBJECTS).await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/archive/2024/many"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn a_board_that_refuses_the_archive_gives_a_page_not_a_crash() {
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
    let response = http().get(format!("{base}/archive")).send().await.unwrap();
    assert_eq!(response.status(), 503);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("The board is not answering")
    );
}

#[tokio::test]
async fn the_way_to_the_archive_is_offered_from_every_page() {
    let (board_url, _) = board(MONTHS, SUBJECTS).await;
    let base = front(&board_url).await;
    let page = page(&base, "/").await;
    assert!(page.contains("<a href=\"/archive\">Archive</a>"), "{page}");
}

#[tokio::test]
async fn the_titles_of_the_subjects_of_a_month_stay_words() {
    let (board_url, _) = board(
        MONTHS,
        concat!(
            "{\"items\":[",
            "{\"id\":\"1\",\"section_slug\":\"general\",",
            "\"title\":\"<script>alert(1)</script>\",\"author_username\":\"alice\",",
            "\"created_at\":\"2024-06-07T10:11:12Z\",\"tags\":[],\"sticky\":false,",
            "\"resolved\":false,\"deleted\":false,\"pending\":false,\"draft\":false,",
            "\"postscore\":0}],",
            "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
            "\"has_next\":false,\"has_previous\":false}}"
        ),
    )
    .await;
    let base = front(&board_url).await;
    let page = page(&base, "/archive/2024/6").await;
    assert!(
        front::html::scripting_free(&page),
        "not scripting free: {page}"
    );
    assert!(page.contains("&lt;script&gt;"), "{page}");
}
