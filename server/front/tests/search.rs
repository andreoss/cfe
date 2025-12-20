use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const HITS: &str = concat!(
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

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn targets(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(status: &'static str, hits: &'static str) -> (String, Log) {
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
                let (answer, body) = match path.as_str() {
                    "/api/search" => (status, hits),
                    "/api/sections" => ("200 OK", "[]"),
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

#[tokio::test]
async fn a_search_page_carries_the_form_and_the_words_it_was_given() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=adapters"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>Search</h2>"));
    assert!(page.contains("action=\"/search\""));
    assert!(page.contains("name=\"q\""));
    assert!(page.contains("value=\"adapters\""));
}

#[tokio::test]
async fn the_words_the_narrowing_and_the_order_are_asked_of_the_board() {
    let (board_url, log) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    http()
        .get(format!(
            "{base}/search?q=adapters&scope=topics&order=oldest"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        log.targets(),
        vec!["/api/search?q=adapters&scope=topics&order=oldest".to_owned()]
    );
}

#[tokio::test]
async fn a_subject_hit_points_at_the_subject_it_is() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=adapters"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h3>2 hits</h3>"));
    assert!(page.contains("<li class=\"hit hit-topic\">"), "{page}");
    assert!(
        page.contains("<span class=\"kind\">Subject</span>"),
        "{page}"
    );
    assert!(
        page.contains(
            "<a href=\"/topics/11111111-1111-1111-1111-111111111111\">Ports and adapters</a>"
        ),
        "{page}"
    );
    assert!(page.contains(">alice<"));
    assert!(page.contains("<time datetime=\"2024-06-07T10:11:12Z\">2024-06-07 10:11</time>"));
}

#[tokio::test]
async fn a_remark_hit_points_at_the_remark_inside_its_subject() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=adapters"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("<span class=\"kind\">Remark</span>"),
        "{page}"
    );
    assert!(
        page.contains(
            "<a href=\"/topics/11111111-1111-1111-1111-111111111111#remark-33333333-3333-3333-3333-333333333333\">Adapters keep the domain clean</a>"
        ),
        "{page}"
    );
    assert!(page.contains(">bob<"));
}

#[tokio::test]
async fn the_words_the_narrowing_and_the_order_stay_on_the_page() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/search?q=adapters&scope=comments&order=newest"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("value=\"/search?q=adapters&amp;scope=comments&amp;order=newest\""),
        "{page}"
    );
    assert!(page.contains("<option value=\"comments\" selected>Remarks</option>"));
    assert!(page.contains("<option value=\"newest\" selected>Newest first</option>"));
}

#[tokio::test]
async fn a_search_without_words_asks_the_board_nothing() {
    let (board_url, log) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(log.targets().is_empty(), "{:?}", log.targets());
    assert!(page.contains("action=\"/search\""));
    assert!(page.contains("value=\"\""));
}

#[tokio::test]
async fn a_search_nobody_wrote_anything_for_says_so() {
    let (board_url, _) = board("200 OK", "[]").await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=nothing"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h3>0 hits</h3>"));
    assert!(page.contains("Nothing was written for those words."));
}

#[tokio::test]
async fn a_narrowing_nobody_knows_falls_back_to_the_wide_one() {
    let (board_url, log) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    http()
        .get(format!(
            "{base}/search?q=adapters&scope=everywhere&order=loudest"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        log.targets(),
        vec!["/api/search?q=adapters&scope=everything&order=relevance".to_owned()]
    );
}

#[tokio::test]
async fn the_words_are_kept_as_words_and_the_page_carries_no_scripting() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=%3Cscript%3Ealert(1)%3C/script%3E"))
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
    assert!(page.contains("&lt;script&gt;"));
}

#[tokio::test]
async fn a_search_can_be_narrowed_without_the_form() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=adapters"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("Narrow to:"), "{page}");
    assert!(
        page.contains(
            "<a href=\"/search?q=adapters&amp;scope=topics&amp;order=relevance\">Subjects</a>"
        ),
        "{page}"
    );
    assert!(
        page.contains(
            "<a href=\"/search?q=adapters&amp;scope=comments&amp;order=relevance\">Remarks</a>"
        ),
        "{page}"
    );
    assert!(
        page.contains(
            "<a href=\"/search?q=adapters&amp;scope=everything&amp;order=relevance\" class=\"current\" aria-current=\"true\">Everything</a>"
        ),
        "{page}"
    );
}

#[tokio::test]
async fn the_hits_can_be_reordered_without_the_form() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/search?q=adapters&scope=comments&order=oldest"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("Order:"), "{page}");
    assert!(
        page.contains(
            "<a href=\"/search?q=adapters&amp;scope=comments&amp;order=newest\">Newest first</a>"
        ),
        "{page}"
    );
    assert!(
        page.contains(
            "<a href=\"/search?q=adapters&amp;scope=comments&amp;order=oldest\" class=\"current\" aria-current=\"true\">Oldest first</a>"
        ),
        "{page}"
    );
}

#[tokio::test]
async fn the_words_stay_inside_the_address_of_every_narrowing() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!("{base}/search?q=a%2Fb%3Fc%20d"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("<a href=\"/search?q=a%2Fb%3Fc%20d&amp;scope=topics&amp;order=relevance\">"),
        "{page}"
    );
}

#[tokio::test]
async fn every_page_offers_the_way_to_the_search_page() {
    let (board_url, _) = board("200 OK", HITS).await;
    let base = front(&board_url).await;
    for address in ["/", "/search"] {
        let page = http()
            .get(format!("{base}{address}"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(page.contains("<a href=\"/search\">Search</a>"), "{page}");
    }
}

#[tokio::test]
async fn a_board_that_refuses_a_search_gives_a_page_not_a_crash() {
    let (board_url, _) = board("500 Internal Server Error", "{\"error\":\"broken\"}").await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/search?q=adapters"))
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
