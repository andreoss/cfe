use front::config;
use front::config::Config;
use front::routes;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SUBJECT: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"group_slug\":null,",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"tags\":[\"rust\"],\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"postscore\":0,",
    "\"pending\":false,\"draft\":false,\"sticky\":false,",
    "\"off_front\":false,\"resolved\":false,\"minor\":false,",
    "\"open_reports\":0}"
);

const REMARKS: &str = concat!(
    "{\"items\":[{\"id\":\"22222222-2222-2222-2222-222222222222\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",\"parent_id\":null,",
    "\"body\":\"a remark\",\"author_username\":\"bob\",",
    "\"created_at\":\"2024-06-07T11:00:00Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"ignored\":false}],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":1,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const SECTIONS: &str =
    "[{\"slug\":\"general\",\"title\":\"General\",\"topics_score\":\"0\",\"may_post\":true}]";

const POLL: &str = concat!(
    "{\"id\":\"77777777-7777-7777-7777-777777777777\",",
    "\"topic_id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"question\":\"Which way?\",\"options\":[",
    "{\"id\":\"88888888-8888-8888-8888-888888888888\",\"text\":\"Ports\",\"votes\":2},",
    "{\"id\":\"99999999-9999-9999-9999-999999999999\",\"text\":\"Adapters\",\"votes\":1}],",
    "\"mine\":null,\"total_votes\":3}"
);

const REACTIONS: &str = concat!(
    "{\"counts\":[{\"kind\":\"like\",\"count\":2},",
    "{\"kind\":\"thanks\",\"count\":1}],\"mine\":null}"
);

const REMARK_REACTIONS: &str = "{\"counts\":[{\"kind\":\"agree\",\"count\":4}],\"mine\":null}";

const REFUSED: &str = "{\"error\":\"not the author\"}";

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const REMARK_ID: &str = "22222222-2222-2222-2222-222222222222";
const OPTION_ID: &str = "88888888-8888-8888-8888-888888888888";

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

async fn board(with_poll: bool) -> (String, Log) {
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
                let mut buffer = vec![0u8; 16384];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let mut parts = request.split_whitespace();
                let method = parts.next().unwrap_or("GET").to_owned();
                let target = parts.next().unwrap_or("/").to_owned();
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
                sink.lock().unwrap().push(format!("{method} {target}"));
                if !body.is_empty() {
                    sink.lock().unwrap().push(format!("BODY {body}"));
                }
                let path = target.split('?').next().unwrap_or("").to_owned();
                let topic = "/api/topics/11111111-1111-1111-1111-111111111111";
                let (status, body) = match path.as_str() {
                    "/api/me" => ("200 OK", user("user")),
                    "/api/sections" => ("200 OK", SECTIONS.to_owned()),
                    _ if path == format!("{topic}/comments") => ("200 OK", REMARKS.to_owned()),
                    _ if path == format!("{topic}/poll") => match (with_poll, method.as_str()) {
                        (true, "POST") => ("200 OK", POLL.to_owned()),
                        (true, _) => ("200 OK", POLL.to_owned()),
                        (false, "POST") => ("200 OK", POLL.to_owned()),
                        _ => ("404 Not Found", REFUSED.to_owned()),
                    },
                    _ if path == format!("{topic}/poll/vote") => ("200 OK", POLL.to_owned()),
                    _ if path == format!("{topic}/reactions") => ("200 OK", REACTIONS.to_owned()),
                    _ if path
                        == format!(
                            "{topic}/comments/22222222-2222-2222-2222-222222222222/reactions"
                        ) =>
                    {
                        ("200 OK", REMARK_REACTIONS.to_owned())
                    }
                    _ if path == topic => ("200 OK", SUBJECT.to_owned()),
                    _ => ("200 OK", "[]".to_owned()),
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

async fn form(base: &str, page: &str, action: &str, fields: &[(&str, &str)]) -> reqwest::Response {
    let token = token_of(base, page).await;
    let mut pairs: Vec<(&str, &str)> = vec![("token", token.as_str())];
    pairs.extend_from_slice(fields);
    http()
        .post(format!("{base}{action}"))
        .header("cookie", format!("session=token; token={token}"))
        .form(&pairs)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn a_subject_page_carries_the_poll_and_its_options() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("<h3>Which way?</h3>"), "{page}");
    assert!(page.contains(">Ports (2)</button>"), "{page}");
    assert!(page.contains(">Adapters (1)</button>"), "{page}");
    assert!(page.contains("3 votes"), "{page}");
    assert!(page.contains(&format!("value=\"{OPTION_ID}\"")), "{page}");
    assert!(
        page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/poll/vote\""),
        "{page}"
    );
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn a_poll_is_shown_to_a_reader_with_no_account_without_the_way_to_vote() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains(">Ports (2)</button>"), "{page}");
    assert!(page.contains("disabled"), "{page}");
    assert!(!page.contains("poll/vote"), "{page}");
}

#[tokio::test]
async fn a_subject_with_no_poll_offers_the_way_to_put_one_up() {
    let (board_url, _) = board(false).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("href=\"/topics/11111111-1111-1111-1111-111111111111/poll/new\""),
        "{page}"
    );
    assert!(page.contains(">Put a poll up<"), "{page}");
    assert!(!page.contains("class=\"poll\""), "{page}");
}

#[tokio::test]
async fn the_way_to_put_a_poll_up_is_offered_to_nobody_else() {
    let (board_url, _) = board(false).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(!page.contains(">Put a poll up<"), "{page}");
}

#[tokio::test]
async fn the_page_to_put_a_poll_up_carries_a_token_and_two_fields() {
    let (board_url, _) = board(false).await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/poll/new"),
        Some("token"),
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(page.contains("name=\"question\""), "{page}");
    assert!(page.contains("name=\"options\""), "{page}");
    assert!(
        page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/poll\""),
        "{page}"
    );
    assert!(page.contains("name=\"token\" value=\""), "{page}");
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn a_poll_put_up_answers_with_the_way_to_the_subject() {
    let (board_url, log) = board(false).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/poll/new"),
        &format!("/topics/{SUBJECT_ID}/poll"),
        &[("question", "Which way?"), ("options", "Ports\nAdapters")],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/topics/11111111-1111-1111-1111-111111111111")
    );
    let calls = log.calls();
    assert!(
        calls
            .iter()
            .any(|call| call == "POST /api/topics/11111111-1111-1111-1111-111111111111/poll"),
        "{calls:?}"
    );
    assert!(
        calls.iter().any(|call| call
            == "BODY {\"question\":\"Which way?\",\"options\":[\"Ports\",\"Adapters\"]}"),
        "{calls:?}"
    );
}

#[tokio::test]
async fn a_poll_without_two_ways_to_answer_is_refused() {
    let (board_url, log) = board(false).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/poll/new"),
        &format!("/topics/{SUBJECT_ID}/poll"),
        &[("question", "Which way?"), ("options", "Ports")],
    )
    .await;
    assert_eq!(response.status().as_u16(), 422);
    let page = response.text().await.unwrap();
    assert!(page.contains("at least two ways to answer"), "{page}");
    assert!(
        !log.calls()
            .iter()
            .any(|call| call.ends_with("/poll") && call.starts_with("POST")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_vote_is_cast_on_the_option_that_was_chosen() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/poll/vote"),
        &[("option", OPTION_ID)],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    let calls = log.calls();
    assert!(
        calls
            .iter()
            .any(|call| call == "POST /api/topics/11111111-1111-1111-1111-111111111111/poll/vote"),
        "{calls:?}"
    );
    assert!(
        calls
            .iter()
            .any(|call| call == &format!("BODY {{\"option_id\":\"{OPTION_ID}\"}}")),
        "{calls:?}"
    );
}

#[tokio::test]
async fn a_vote_without_the_token_of_its_page_is_refused() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/poll/vote"))
        .header("cookie", "session=token")
        .form(&[("option", OPTION_ID)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
    assert!(
        !log.calls().iter().any(|call| call.contains("poll/vote")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_subject_carries_the_reactions_of_everybody_and_the_way_to_react() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains(">like 2</button>"), "{page}");
    assert!(page.contains(">thanks 1</button>"), "{page}");
    assert!(page.contains(">agree 0</button>"), "{page}");
    assert!(
        page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/react\""),
        "{page}"
    );
    assert!(
        page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/comments/22222222-2222-2222-2222-222222222222/react\""),
        "{page}"
    );
    assert!(page.contains(">agree 4</button>"), "{page}");
}

#[tokio::test]
async fn a_reader_with_no_account_is_offered_no_way_to_react() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains(">like 2</button>"), "{page}");
    assert!(!page.contains("/react\""), "{page}");
    assert!(page.contains("disabled"), "{page}");
}

#[tokio::test]
async fn reacting_to_a_subject_answers_with_the_way_back_to_it() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/react"),
        &[("kind", "like")],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    let calls = log.calls();
    assert!(
        calls
            .iter()
            .any(|call| call == "POST /api/topics/11111111-1111-1111-1111-111111111111/reactions"),
        "{calls:?}"
    );
    assert!(
        calls.iter().any(|call| call == "BODY {\"kind\":\"like\"}"),
        "{calls:?}"
    );
}

#[tokio::test]
async fn a_reaction_nobody_knows_is_not_sent_to_the_board() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/react"),
        &[("kind", "shrug")],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    assert!(
        !log.calls()
            .iter()
            .any(|call| call.contains("/reactions") && call.starts_with("POST")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_reaction_is_taken_away_again() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/reactions/clear"),
        &[],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    assert!(
        log.calls().iter().any(|call| call
            == "DELETE /api/topics/11111111-1111-1111-1111-111111111111/reactions"),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_remark_is_reacted_to_and_its_reaction_taken_away() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/react"),
        &[("kind", "thanks")],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    assert!(
        log.calls().iter().any(|call| call == &format!(
            "POST /api/topics/11111111-1111-1111-1111-111111111111/comments/{REMARK_ID}/reactions"
        )),
        "{:?}",
        log.calls()
    );
    let response = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/comments/{REMARK_ID}/reactions/clear"),
        &[],
    )
    .await;
    assert_eq!(response.status().as_u16(), 303);
    assert!(
        log.calls().iter().any(|call| call == &format!(
            "DELETE /api/topics/11111111-1111-1111-1111-111111111111/comments/{REMARK_ID}/reactions"
        )),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn reacting_without_the_token_of_the_page_is_refused() {
    let (board_url, log) = board(true).await;
    let base = front(&board_url).await;
    let response = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/react"))
        .header("cookie", "session=token")
        .form(&[("kind", "like")])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
    assert!(
        !log.calls()
            .iter()
            .any(|call| call.contains("/reactions") && call.starts_with("POST")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in_before_reacting() {
    let (board_url, _) = board(true).await;
    let base = front(&board_url).await;
    let token = {
        let response = get(&base, &format!("/topics/{SUBJECT_ID}"), None).await;
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
    };
    let response = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/react"))
        .header("cookie", format!("token={token}"))
        .form(&[("token", token.as_str()), ("kind", "like")])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/sign-in?return_to=/topics/11111111-1111-1111-1111-111111111111")
    );
}
