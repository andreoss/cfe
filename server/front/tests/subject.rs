use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SUBJECT: &str = "{\"id\":\"11111111-1111-1111-1111-111111111111\",\"section_slug\":\"general\",\"group_slug\":null,\"title\":\"First subject\",\"body\":\"The opening remark\",\"tags\":[\"rust\"],\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":true,\"postscore\":2,\"pending\":false,\"draft\":false,\"sticky\":false,\"off_front\":false,\"resolved\":true,\"minor\":false,\"open_reports\":0}";

const COMMENTS: &str = "{\"items\":[{\"id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"First reply\",\"author_username\":\"bob\",\"created_at\":\"2024-06-07T11:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false},{\"id\":\"bbbbbbbb-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"Second reply\",\"author_username\":\"carol\",\"created_at\":\"2024-06-07T12:00:00Z\",\"deleted\":true,\"deleted_reason\":\"off topic\",\"edited\":false,\"ignored\":false}],\"page\":{\"number\":1,\"size\":25,\"total\":2,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}";

const REPLIES: &str = "{\"items\":[{\"id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"First reply\",\"author_username\":\"bob\",\"created_at\":\"2024-06-07T11:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false},{\"id\":\"bbbbbbbb-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"body\":\"An answer to the first\",\"author_username\":\"carol\",\"created_at\":\"2024-06-07T12:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false}],\"page\":{\"number\":1,\"size\":25,\"total\":2,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}";

const NO_COMMENTS: &str = "{\"items\":[],\"page\":{\"number\":1,\"size\":25,\"total\":0,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}";

const MARKUP: &str = "{\"id\":\"11111111-1111-1111-1111-111111111111\",\"section_slug\":\"general\",\"group_slug\":null,\"title\":\"<script>alert(1)</script>\",\"body\":\"<img src=1 onerror=alert(1)>\",\"tags\":[\"<b>\"],\"author_username\":\"<i>alice\",\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"postscore\":0,\"pending\":false,\"draft\":false,\"sticky\":false,\"off_front\":false,\"resolved\":false,\"minor\":false,\"open_reports\":0}";

const RENDERED: &str = "{\"id\":\"11111111-1111-1111-1111-111111111111\",\"section_slug\":\"general\",\"group_slug\":null,\"title\":\"First subject\",\"body\":\"# Heading\\n\\nA **bold** claim and a [link](https://example.org).\",\"tags\":[\"rust\"],\"author_username\":\"alice\",\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"postscore\":0,\"pending\":false,\"draft\":false,\"sticky\":false,\"off_front\":false,\"resolved\":false,\"minor\":false,\"open_reports\":0}";

const RENDERED_COMMENTS: &str = "{\"items\":[{\"id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"> spoken\\n\\n- one\\n- two\",\"author_username\":\"bob\",\"created_at\":\"2024-06-07T11:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false},{\"id\":\"bbbbbbbb-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"An answer with `code`\",\"author_username\":\"carol\",\"created_at\":\"2024-06-07T12:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false}],\"page\":{\"number\":1,\"size\":25,\"total\":2,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}";

const QUOTED: &str = "{\"items\":[{\"id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,\"body\":\"Spoken by bob\",\"author_username\":\"bob\",\"created_at\":\"2024-06-07T11:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false},{\"id\":\"bbbbbbbb-0000-0000-0000-000000000000\",\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":\"aaaaaaaa-0000-0000-0000-000000000000\",\"body\":\"Agreed, @carol says so\",\"author_username\":\"carol\",\"created_at\":\"2024-06-07T12:00:00Z\",\"deleted\":false,\"deleted_reason\":null,\"edited\":false,\"ignored\":false}],\"page\":{\"number\":1,\"size\":25,\"total\":2,\"total_pages\":1,\"has_next\":false,\"has_previous\":false}}";

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn targets(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(subject: &'static str, comments: &'static str) -> (String, Log) {
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
                let (status, body) = if path == "/api/topics/11111111-1111-1111-1111-111111111111" {
                    ("200 OK", subject)
                } else if path == "/api/topics/11111111-1111-1111-1111-111111111111/comments" {
                    ("200 OK", comments)
                } else {
                    ("404 Not Found", "{\"error\":\"not found\"}")
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
async fn a_subject_page_shows_the_remark_and_the_remarks_under_it() {
    let (board_url, _) = board(SUBJECT, COMMENTS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h2>First subject</h2>"));
    assert!(page.contains(">alice<"));
    assert!(page.contains("datetime=\"2024-06-07T10:11:12Z\""));
    assert!(page.contains(">2024-06-07 10:11<"));
    assert!(page.contains("The opening remark"));
    assert!(page.contains("href=\"/tags/rust\""));
    assert!(page.contains("href=\"/sections/general\""));
    assert!(page.contains(">bob<"));
    assert!(page.contains("First reply"));
    assert!(page.contains(">carol<"));
    assert!(page.contains("(resolved, edited)"), "no marks: {page}");
    assert!(page.contains("Removed: off topic"));
}

#[tokio::test]
async fn a_reply_is_set_under_the_remark_it_answers() {
    let (board_url, _) = board(SUBJECT, REPLIES).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let answered = page
        .find("id=\"remark-aaaaaaaa-0000-0000-0000-000000000000\"")
        .unwrap();
    let answer = page
        .find("id=\"remark-bbbbbbbb-0000-0000-0000-000000000000\"")
        .unwrap();
    assert!(answered < answer);
    assert!(
        page[answered..answer].contains("<ol>"),
        "the answer opens a list inside the remark it answers: {page}"
    );
    assert!(page.contains("An answer to the first"));
    assert_eq!(page.matches("<ol").count(), page.matches("</ol>").count());
}

#[tokio::test]
async fn a_subject_with_no_remarks_says_so() {
    let (board_url, _) = board(SUBJECT, NO_COMMENTS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("No remarks here yet."));
}

#[tokio::test]
async fn a_subject_page_carries_no_scripting_and_keeps_its_own_address() {
    let (board_url, _) = board(SUBJECT, COMMENTS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111?page=2"
        ))
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
    assert!(page.contains("value=\"/topics/11111111-1111-1111-1111-111111111111?page=2\""));
}

#[tokio::test]
async fn a_subject_that_is_not_there_is_a_page_and_not_a_crash() {
    let (board_url, _) = board(SUBJECT, COMMENTS).await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!(
            "{base}/topics/33333333-3333-3333-3333-333333333333"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert!(response.text().await.unwrap().contains("No such subject"));
}

#[tokio::test]
async fn a_subject_page_asks_the_board_for_the_page_in_the_address() {
    let (board_url, log) = board(SUBJECT, COMMENTS).await;
    let base = front(&board_url).await;
    http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111?page=3"
        ))
        .send()
        .await
        .unwrap();
    assert!(
        log.targets().contains(
            &"/api/topics/11111111-1111-1111-1111-111111111111/comments?page=3".to_owned()
        )
    );
}

#[tokio::test]
async fn text_from_the_board_cannot_become_markup_on_a_subject_page() {
    let (board_url, _) = board(MARKUP, COMMENTS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains("<script"));
    assert!(!page.contains("<img"));
    assert!(!page.contains("<b>"));
    assert!(page.contains("&lt;script&gt;"));
    assert!(
        front::html::scripting_free(&page),
        "not scripting free: {page}"
    );
}

#[tokio::test]
async fn a_reply_carries_a_quote_of_what_it_answers() {
    let (board_url, _) = board(SUBJECT, QUOTED).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.contains(
            "<a href=\"#remark-aaaaaaaa-0000-0000-0000-000000000000\">bob wrote</a>: Spoken by bob"
        ),
        "no quote of what it answers: {page}"
    );
    assert!(
        page.contains("<a class=\"mention\" href=\"/u/carol\">@carol</a>"),
        "the account it names is not linked: {page}"
    );
    assert!(
        front::html::scripting_free(&page),
        "not scripting free: {page}"
    );
}

#[tokio::test]
async fn a_subject_and_its_remarks_come_back_rendered() {
    let (board_url, _) = board(RENDERED, RENDERED_COMMENTS).await;
    let base = front(&board_url).await;
    let page = http()
        .get(format!(
            "{base}/topics/11111111-1111-1111-1111-111111111111"
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h4>Heading</h4>"), "no heading: {page}");
    assert!(page.contains("<strong>bold</strong>"), "no strong: {page}");
    assert!(
        page.contains("<a href=\"https://example.org\">link</a>"),
        "no link: {page}"
    );
    assert!(page.contains("<blockquote>"), "no quote: {page}");
    assert!(page.contains("<li>one</li>"), "no item: {page}");
    assert!(page.contains("<code>code</code>"), "no code: {page}");
    assert!(
        !page.contains("# Heading"),
        "the source was left in: {page}"
    );
    assert!(!page.contains("**bold**"), "the source was left in: {page}");
    assert!(
        front::html::scripting_free(&page),
        "not scripting free: {page}"
    );
}
