use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECTIONS: &str =
    "[{\"slug\":\"general\",\"title\":\"General\",\"topics_score\":\"10\",\"may_post\":true}]";

const GROUPS: &str = concat!(
    "[{\"id\":\"cccccccc-cccc-cccc-cccc-cccccccccccc\",",
    "\"section_slug\":\"general\",\"name\":\"Writers\",\"slug\":\"writers\"}]"
);

const MADE_SECTION: &str = "{\"slug\":\"tech\",\"title\":\"Technology\",\"topics_score\":\"0\"}";
const RENAMED_SECTION: &str =
    "{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"10\"}";
const SCORED: &str = "{\"slug\":\"general\",\"title\":\"General\",\"topics_score\":\"20\"}";
const MADE_GROUP: &str = concat!(
    "{\"id\":\"dddddddd-dddd-dddd-dddd-dddddddddddd\",",
    "\"section_slug\":\"general\",\"name\":\"Readers\",\"slug\":\"readers\"}"
);
const RENAMED_GROUP: &str = concat!(
    "{\"id\":\"cccccccc-cccc-cccc-cccc-cccccccccccc\",",
    "\"section_slug\":\"general\",\"name\":\"Writers of prose\",\"slug\":\"writers\"}"
);
const MAINTENANCE: &str = "{\"blocked\":1,\"dropped\":2}";

fn user(role: &str) -> String {
    format!("{{\"id\":\"9\",\"username\":\"alice\",\"role\":\"{role}\"}}")
}

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn calls(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

async fn board(role: &str) -> (String, Log) {
    let role = role.to_owned();
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
            let role = role.clone();
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 16384];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let mut parts = request.split_whitespace();
                let method = parts.next().unwrap_or("GET").to_owned();
                let target = parts.next().unwrap_or("/").to_owned();
                sink.lock().unwrap().push(format!("{method} {target}"));
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (status, body) = match (path.as_str(), method.as_str()) {
                    ("/api/me", _) => ("200 OK", user(&role)),
                    ("/api/sections", "GET") => ("200 OK", SECTIONS.to_owned()),
                    ("/api/sections", "POST") => ("200 OK", MADE_SECTION.to_owned()),
                    ("/api/sections/general/groups", "GET") => ("200 OK", GROUPS.to_owned()),
                    ("/api/sections/general/groups", "POST") => ("200 OK", MADE_GROUP.to_owned()),
                    ("/api/sections/general/settings", "PATCH") => {
                        ("200 OK", RENAMED_SECTION.to_owned())
                    }
                    ("/api/sections/general/topics-score", "POST") => ("200 OK", SCORED.to_owned()),
                    ("/api/sections/general/groups/writers", "PATCH") => {
                        ("200 OK", RENAMED_GROUP.to_owned())
                    }
                    ("/api/maintenance/run", "POST") => ("200 OK", MAINTENANCE.to_owned()),
                    _ => ("404 Not Found", "{\"error\":\"no\"}".to_owned()),
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

async fn get(base: &str, address: &str, session: Option<&str>) -> reqwest::Response {
    let request = http().get(format!("{base}{address}"));
    let request = match session {
        Some(token) => request.header("cookie", format!("session={token}")),
        None => request,
    };
    request.send().await.unwrap()
}

async fn token_of(base: &str, address: &str) -> String {
    let response = get(base, address, Some("token")).await;
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

async fn post_form(
    base: &str,
    action: &str,
    fields: &[(&str, &str)],
    token: Option<&str>,
) -> reqwest::Response {
    let cookie = match token {
        Some(token) => format!("session=token; token={token}"),
        None => "session=token".to_owned(),
    };
    http()
        .post(format!("{base}{action}"))
        .header("cookie", cookie)
        .form(&fields.to_vec())
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn the_way_in_to_administration() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let page = get(&base, "/admin", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("Administration"), "{page}");
    assert!(page.contains("action=\"/admin/sections\""), "{page}");
    assert!(
        page.contains("action=\"/admin/sections/general/rename\""),
        "{page}"
    );
    assert!(
        page.contains("action=\"/admin/sections/general/score\""),
        "{page}"
    );
    assert!(
        page.contains("action=\"/admin/sections/general/groups\""),
        "{page}"
    );
    assert!(
        page.contains("action=\"/admin/sections/general/groups/writers/rename\""),
        "{page}"
    );
    assert!(page.contains("action=\"/admin/maintenance\""), "{page}");
    assert!(page.contains("Writers"), "{page}");
    assert!(page.contains("writers"), "{page}");
    assert!(page.contains("10"), "{page}");
    assert!(!page.contains("<script"), "{page}");
    assert!(log.calls().contains(&"GET /api/sections".to_owned()));
    assert!(
        log.calls()
            .contains(&"GET /api/sections/general/groups".to_owned())
    );
}

#[tokio::test]
async fn a_reader_with_no_account_goes_to_sign_in() {
    let (board_url, _) = board("moderator").await;
    let base = front(&board_url).await;
    let response = get(&base, "/admin", None).await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/sign-in?return_to=/admin")
    );
}

#[tokio::test]
async fn an_account_that_does_not_keep_the_board_is_offered_none_of_the_ways() {
    let (board_url, log) = board("user").await;
    let base = front(&board_url).await;
    let page = get(&base, "/admin", Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(!page.contains("action=\"/admin/sections\""), "{page}");
    assert!(!page.contains("admin/maintenance"), "{page}");
    assert!(
        !log.calls().iter().any(|call| call.contains("POST /api")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_section_is_made_from_the_page() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/admin").await;
    let response = post_form(
        &base,
        "/admin/sections",
        &[("token", &token), ("slug", "tech"), ("title", "Technology")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/admin")
    );
    assert!(log.calls().contains(&"POST /api/sections".to_owned()));
}

#[tokio::test]
async fn a_section_is_renamed_from_the_page() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/admin").await;
    let response = post_form(
        &base,
        "/admin/sections/general/rename",
        &[("token", &token), ("title", "General Talk")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls()
            .contains(&"PATCH /api/sections/general/settings".to_owned())
    );
}

#[tokio::test]
async fn the_score_of_a_section_is_set_from_the_page() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/admin").await;
    let response = post_form(
        &base,
        "/admin/sections/general/score",
        &[("token", &token), ("score", "20")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls()
            .contains(&"POST /api/sections/general/topics-score".to_owned())
    );
}

#[tokio::test]
async fn a_group_is_made_from_the_page() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/admin").await;
    let response = post_form(
        &base,
        "/admin/sections/general/groups",
        &[("token", &token), ("name", "Readers"), ("slug", "readers")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls()
            .contains(&"POST /api/sections/general/groups".to_owned())
    );
}

#[tokio::test]
async fn a_group_is_renamed_from_the_page() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/admin").await;
    let response = post_form(
        &base,
        "/admin/sections/general/groups/writers/rename",
        &[("token", &token), ("name", "Writers of prose")],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert!(
        log.calls()
            .contains(&"PATCH /api/sections/general/groups/writers".to_owned())
    );
}

#[tokio::test]
async fn the_maintenance_is_run_from_the_page() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let token = token_of(&base, "/admin").await;
    let response = post_form(
        &base,
        "/admin/maintenance",
        &[("token", &token)],
        Some(&token),
    )
    .await;
    assert_eq!(response.status(), 200);
    let page = response.text().await.unwrap();
    assert!(page.contains("blocked"), "{page}");
    assert!(page.contains("dropped"), "{page}");
    assert!(page.contains("1"), "{page}");
    assert!(page.contains("2"), "{page}");
    assert!(
        log.calls()
            .contains(&"POST /api/maintenance/run".to_owned())
    );
}

#[tokio::test]
async fn every_way_without_the_token_of_the_page_is_refused() {
    let (board_url, log) = board("moderator").await;
    let base = front(&board_url).await;
    let actions = [
        "/admin/sections",
        "/admin/sections/general/rename",
        "/admin/sections/general/score",
        "/admin/sections/general/groups",
        "/admin/sections/general/groups/writers/rename",
        "/admin/maintenance",
    ];
    for action in actions {
        let response = post_form(&base, action, &[("token", "not-mine")], None).await;
        assert_eq!(response.status(), 403, "{action}");
    }
    let posting = log
        .calls()
        .iter()
        .filter(|call| call.starts_with("POST /api") || call.starts_with("PATCH /api"))
        .count();
    assert_eq!(posting, 0, "{:?}", log.calls());
}
