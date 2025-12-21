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

const NOTICES: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"22222222-2222-2222-2222-222222222222\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"topic_title\":\"Ports and adapters\",",
    "\"comment_id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"actor_username\":\"bob\",\"created_at\":\"2024-06-08T09:08:07Z\",",
    "\"read\":false,\"kind\":\"reply\"}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const NOTICES_READ: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"22222222-2222-2222-2222-222222222222\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"topic_title\":\"Ports and adapters\",",
    "\"comment_id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"actor_username\":\"bob\",\"created_at\":\"2024-06-08T09:08:07Z\",",
    "\"read\":true,\"kind\":\"reply\"}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const NOTICES_WATCHED: &str = concat!(
    "{\"items\":[",
    "{\"id\":\"22222222-2222-2222-2222-222222222222\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"topic_title\":\"Ports and adapters\",",
    "\"comment_id\":\"33333333-3333-3333-3333-333333333333\",",
    "\"actor_username\":\"bob\",\"created_at\":\"2024-06-08T09:08:07Z\",",
    "\"read\":false,\"kind\":\"watch\"}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const NO_NOTICES: &str = concat!(
    "{\"items\":[],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":0,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const TAGS: &str = "[\"rust\",\"adapters\"]";
const NO_TAGS: &str = "[]";

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
    board_full(kept, answer, NOTICES, TAGS).await
}

async fn board_full(
    kept: &'static str,
    answer: &'static str,
    notices: &'static str,
    tags: &'static str,
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
                sink.lock().unwrap().push(format!("{method} {target}"));
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (answer, body) = match path.as_str() {
                    "/api/me" => ("200 OK", USER),
                    "/api/sections" => ("200 OK", "[]"),
                    "/api/bookmarks" | "/api/watched" => (answer, kept),
                    "/api/notifications" => (answer, notices),
                    "/api/followed-tags" => (answer, tags),
                    other if other.starts_with("/api/notifications/") => ("200 OK", "{}"),
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
async fn a_notifications_page_asks_the_board_for_the_notices_and_for_the_tags_followed() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    signed_page(&base, "/notifications").await;
    let calls = log.calls();
    assert!(
        calls.contains(&"GET /api/notifications?page=1".to_owned()),
        "{calls:?}"
    );
    assert!(
        calls.contains(&"GET /api/followed-tags".to_owned()),
        "{calls:?}"
    );
}

#[tokio::test]
async fn a_notifications_page_carries_each_notice_and_where_it_points() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/notifications").await;
    assert!(page.contains("<h2>Notifications</h2>"), "{page}");
    assert!(page.contains("<h3>1 notification</h3>"), "{page}");
    assert!(
        page.contains(
            "<a href=\"/topics/11111111-1111-1111-1111-111111111111\">Ports and adapters</a>"
        ),
        "{page}"
    );
    assert!(page.contains(">bob<"), "{page}");
    assert!(
        page.contains("<time datetime=\"2024-06-08T09:08:07Z\">2024-06-08 09:08</time>"),
        "{page}"
    );
    assert!(page.contains("class=\"notice unread\""), "{page}");
}

#[tokio::test]
async fn an_unread_notice_offers_the_way_to_mark_it_read() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/notifications").await;
    assert!(
        page.contains("action=\"/notifications/22222222-2222-2222-2222-222222222222/read\""),
        "{page}"
    );
    assert!(page.contains("Mark read"), "{page}");
}

#[tokio::test]
async fn a_notice_that_was_read_offers_no_way_to_mark_it() {
    let (board_url, _) = board_full(KEPT, "200 OK", NOTICES_READ, TAGS).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/notifications").await;
    assert!(!page.contains("/read\""), "{page}");
    assert!(page.contains("class=\"notice read\""), "{page}");
}

#[tokio::test]
async fn marking_a_notice_read_asks_the_board_and_comes_back_to_the_page() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/notifications").await;
    let response = http()
        .post(format!(
            "{base}/notifications/22222222-2222-2222-2222-222222222222/read"
        ))
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
            .unwrap()
            .to_str()
            .unwrap(),
        "/notifications"
    );
    assert!(
        log.calls().contains(
            &"POST /api/notifications/22222222-2222-2222-2222-222222222222/read".to_owned()
        ),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_form_that_does_not_carry_the_token_is_refused() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    let response = http()
        .post(format!(
            "{base}/notifications/22222222-2222-2222-2222-222222222222/read"
        ))
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
        !log.calls().iter().any(|call| call.starts_with("POST")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn the_notices_of_a_reader_with_no_session_are_not_asked_for() {
    let (board_url, log) = board(KEPT).await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/notifications"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/sign-in?return_to=/notifications"
    );
    assert!(
        !log.calls()
            .iter()
            .any(|call| call.contains("notifications")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_notifications_list_that_holds_nothing_says_so() {
    let (board_url, _) = board_full(KEPT, "200 OK", NO_NOTICES, NO_TAGS).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/notifications").await;
    assert!(page.contains("<h3>No notifications</h3>"), "{page}");
    assert!(page.contains("Nothing has happened yet."), "{page}");
    assert!(page.contains("No tag is followed yet."), "{page}");
}

#[tokio::test]
async fn the_tags_an_account_follows_are_carried_with_the_way_to_them() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/notifications").await;
    assert!(page.contains("<h3>Followed tags</h3>"), "{page}");
    assert!(page.contains("<a href=\"/tags/rust\">rust</a>"), "{page}");
    assert!(
        page.contains("<a href=\"/tags/adapters\">adapters</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_notice_about_a_remark_says_what_was_done() {
    let (board_url, _) = board_full(KEPT, "200 OK", NOTICES_WATCHED, NO_TAGS).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/notifications").await;
    assert!(page.contains("remarked in"), "{page}");
}

#[tokio::test]
async fn the_notifications_are_offered_to_the_account_that_holds_them() {
    let (board_url, _) = board(KEPT).await;
    let base = front(&board_url).await;
    let page = signed_page(&base, "/bookmarks").await;
    assert!(
        page.contains("<a href=\"/notifications\">Notifications</a>"),
        "{page}"
    );
}

#[tokio::test]
async fn a_board_that_refuses_the_notices_gives_a_page_and_not_a_crash() {
    let (board_url, _) = board_with("{\"error\":\"broken\"}", "500 Internal Server Error").await;
    let base = front(&board_url).await;
    let response = http()
        .get(format!("{base}/notifications"))
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
