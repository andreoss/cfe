use front::config;
use front::config::Config;
use front::routes;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SUBJECT: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"group_slug\":null,",
    "\"title\":\"Ports and adapters\",\"body\":\"the body of it\",",
    "\"tags\":[\"rust\"],\"author_username\":\"alice\",",
    "\"created_at\":\"2024-06-07T10:11:12Z\",\"deleted\":false,",
    "\"deleted_reason\":null,\"edited\":false,\"postscore\":0,",
    "\"pending\":true,\"draft\":true,\"sticky\":false,",
    "\"off_front\":false,\"resolved\":false,\"minor\":false,",
    "\"open_reports\":0}"
);

const REMARKS: &str = concat!(
    "{\"items\":[],",
    "\"page\":{\"number\":1,\"size\":20,\"total\":0,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);

const SECTIONS: &str =
    "[{\"slug\":\"general\",\"title\":\"General\",\"topics_score\":\"0\",\"may_post\":true}]";

const GROUPS: &str = concat!(
    "[{\"id\":\"66666666-6666-6666-6666-666666666666\",",
    "\"section_slug\":\"general\",\"name\":\"Team\",\"slug\":\"team\"}]"
);

const IMAGES: &str = concat!(
    "[{\"id\":\"55555555-5555-5555-5555-555555555555\",",
    "\"content_type\":\"image/png\",\"uploaded_by\":\"alice\"}]"
);

const IMAGE: &str = concat!(
    "{\"id\":\"55555555-5555-5555-5555-555555555555\",",
    "\"content_type\":\"image/png\",\"uploaded_by\":\"alice\"}"
);

const REFUSED: &str = "{\"error\":\"moderator role required\"}";

const SUBJECT_ID: &str = "11111111-1111-1111-1111-111111111111";
const IMAGE_ID: &str = "55555555-5555-5555-5555-555555555555";

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

async fn board(role: &'static str, reason: &'static str) -> (String, Log) {
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
                if !body.is_empty() && !body.contains("multipart") {
                    sink.lock().unwrap().push(format!("BODY {body}"));
                }
                let path = target.split('?').next().unwrap_or("").to_owned();
                let (status, kind, body) = match path.as_str() {
                    "/api/me" => ("200 OK", "application/json", user(role)),
                    "/api/sections" => ("200 OK", "application/json", SECTIONS.to_owned()),
                    "/api/sections/general/groups" => {
                        ("200 OK", "application/json", GROUPS.to_owned())
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111" => {
                        ("200 OK", "application/json", SUBJECT.to_owned())
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/comments" => {
                        ("200 OK", "application/json", REMARKS.to_owned())
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/images" => {
                        match method.as_str() {
                            "POST" => ("201 Created", "application/json", IMAGE.to_owned()),
                            _ => ("200 OK", "application/json", IMAGES.to_owned()),
                        }
                    }
                    "/api/topics/11111111-1111-1111-1111-111111111111/images/55555555-5555-5555-5555-555555555555" => {
                        match method.as_str() {
                            "DELETE" => ("204 No Content", "application/json", String::new()),
                            _ => ("200 OK", "image/png", "PNGDATA".to_owned()),
                        }
                    }
                    _ if path.starts_with("/api/topics/11111111-1111-1111-1111-111111111111/") => {
                        match reason {
                            "200 OK" => ("200 OK", "application/json", SUBJECT.to_owned()),
                            _ => (reason, "application/json", REFUSED.to_owned()),
                        }
                    }
                    _ => (reason, "application/json", REFUSED.to_owned()),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
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
    let mut vars = BTreeMap::new();
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

async fn token_only(base: &str, address: &str) -> String {
    let response = get(base, address, None).await;
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
async fn a_moderator_is_offered_every_way_of_a_subject_life() {
    let (board_url, _) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/publish\""),
        "{page}"
    );
    assert!(page.contains(">Publish<"), "{page}");
    assert!(page.contains(">Pin<"), "{page}");
    assert!(page.contains(">Keep off the front page<"), "{page}");
    assert!(page.contains(">Commit<"), "{page}");
    assert!(page.contains("name=\"score\""), "{page}");
    assert!(page.contains("name=\"group\""), "{page}");
    assert!(page.contains(">Team<"), "{page}");
    assert!(page.contains(">Mark resolved<"), "{page}");
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn an_account_is_offered_only_the_ways_that_are_its_own() {
    let (board_url, _) = board("user", "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains(">Publish<"), "{page}");
    assert!(page.contains(">Mark resolved<"), "{page}");
    assert!(!page.contains(">Pin<"), "{page}");
    assert!(!page.contains("name=\"score\""), "{page}");
    assert!(!page.contains("name=\"group\""), "{page}");
    assert!(!page.contains(">Commit<"), "{page}");
}

#[tokio::test]
async fn a_reader_with_no_account_is_offered_none_of_them() {
    let (board_url, _) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), None)
        .await
        .text()
        .await
        .unwrap();
    assert!(!page.contains(">Publish<"), "{page}");
    assert!(!page.contains(">Pin<"), "{page}");
    assert!(!page.contains(">Mark resolved<"), "{page}");
    assert!(!page.contains("class=\"life\""), "{page}");
}

#[tokio::test]
async fn publishing_a_subject_answers_with_the_way_back_to_it() {
    let (board_url, log) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/publish"),
        &[],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}")
    );
    assert!(
        log.calls()
            .contains(&"POST /api/topics/11111111-1111-1111-1111-111111111111/publish".to_owned()),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn pinning_and_unpinning_carry_what_was_asked_for() {
    let (board_url, log) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/sticky"),
        &[("on", "yes")],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert!(
        log.calls()
            .iter()
            .any(|call| call == "BODY {\"sticky\":true}"),
        "{:?}",
        log.calls()
    );
    form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/sticky"),
        &[("on", "no")],
    )
    .await;
    assert!(
        log.calls()
            .iter()
            .any(|call| call == "BODY {\"sticky\":false}"),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn keeping_a_subject_off_the_front_page_is_the_other_way_round() {
    let (board_url, log) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/front"),
        &[("on", "no")],
    )
    .await;
    assert!(
        log.calls()
            .iter()
            .any(|call| call == "BODY {\"off_front\":true}"),
        "{:?}",
        log.calls()
    );
    assert!(
        log.calls().contains(
            &"POST /api/topics/11111111-1111-1111-1111-111111111111/off-front".to_owned()
        ),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_score_and_a_group_are_taken_from_the_form() {
    let (board_url, log) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/score"),
        &[("score", "-2")],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert!(
        log.calls()
            .iter()
            .any(|call| call.contains("\"postscore\":-2")),
        "{:?}",
        log.calls()
    );
    form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/group"),
        &[("group", "team")],
    )
    .await;
    assert!(
        log.calls()
            .iter()
            .any(|call| call.contains("\"group\":\"team\"")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_way_of_a_subject_life_without_the_token_of_its_page_is_refused() {
    let (board_url, log) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let answer = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/publish"))
        .header("cookie", "session=token")
        .form(&[("token", "not-the-one")])
        .send()
        .await
        .unwrap();
    assert_eq!(answer.status(), 403);
    assert!(
        !log.calls().iter().any(|call| call.contains("/publish")),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_reader_with_no_account_is_sent_to_sign_in_before_a_way_of_life() {
    let (board_url, _) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let token = token_only(&base, &format!("/topics/{SUBJECT_ID}")).await;
    let answer = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/publish"))
        .header("cookie", format!("token={token}"))
        .form(&[("token", token.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/sign-in?return_to=/topics/{SUBJECT_ID}")
    );
}

#[tokio::test]
async fn a_refusal_from_the_board_comes_back_in_words() {
    let (board_url, _) = board("moderator", "403 Forbidden").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}"),
        &format!("/topics/{SUBJECT_ID}/sticky"),
        &[("on", "yes")],
    )
    .await;
    assert_eq!(answer.status(), 422);
    let page = answer.text().await.unwrap();
    assert!(page.contains("moderator role required"), "{page}");
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn the_pictures_of_a_subject_are_shown_on_its_page_and_served_as_bytes() {
    let (board_url, _) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let page = get(&base, &format!("/topics/{SUBJECT_ID}"), Some("token"))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        page.contains("src=\"/topics/11111111-1111-1111-1111-111111111111/images/55555555-5555-5555-5555-555555555555\""),
        "{page}"
    );
    assert!(page.contains("alt=\"A picture put up by alice\""), "{page}");
    let picture = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/images/{IMAGE_ID}"),
        Some("token"),
    )
    .await;
    assert_eq!(picture.status(), 200);
    assert_eq!(picture.headers().get("content-type").unwrap(), "image/png");
    assert_eq!(picture.bytes().await.unwrap().as_ref(), b"PNGDATA");
}

#[tokio::test]
async fn the_page_of_pictures_offers_the_way_to_put_one_up_and_to_take_one_down() {
    let (board_url, _) = board("moderator", "200 OK").await;
    let base = front(&board_url).await;
    let page = get(
        &base,
        &format!("/topics/{SUBJECT_ID}/images"),
        Some("token"),
    )
    .await
    .text()
    .await
    .unwrap();
    assert!(
        page.contains("<h2>Pictures of Ports and adapters</h2>"),
        "{page}"
    );
    assert!(
        page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/images\"")
            && page.contains("enctype=\"multipart/form-data\""),
        "{page}"
    );
    assert!(page.contains("name=\"picture\""), "{page}");
    assert!(
        page.contains(&format!("/topics/{SUBJECT_ID}/images/{IMAGE_ID}/remove")),
        "{page}"
    );
    assert!(!page.contains("<script"), "{page}");
}

#[tokio::test]
async fn taking_a_picture_down_answers_with_the_way_back_to_them() {
    let (board_url, log) = board("moderator", "204 No Content").await;
    let base = front(&board_url).await;
    let answer = form(
        &base,
        &format!("/topics/{SUBJECT_ID}/images"),
        &format!("/topics/{SUBJECT_ID}/images/{IMAGE_ID}/remove"),
        &[],
    )
    .await;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get("location").unwrap(),
        &format!("/topics/{SUBJECT_ID}/images")
    );
    assert!(
        log.calls().contains(
            &"DELETE /api/topics/11111111-1111-1111-1111-111111111111/images/55555555-5555-5555-5555-555555555555".to_owned()
        ),
        "{:?}",
        log.calls()
    );
}

#[tokio::test]
async fn a_picture_is_put_up_from_the_browser() {
    let (board_url, log) = board("moderator", "201 Created").await;
    let base = front(&board_url).await;
    let token = token_of(&base, &format!("/topics/{SUBJECT_ID}/images")).await;
    let part = reqwest::multipart::Part::bytes(b"PNGDATA".to_vec())
        .file_name("shot.png")
        .mime_str("image/png")
        .unwrap();
    let answer = http()
        .post(format!("{base}/topics/{SUBJECT_ID}/images"))
        .header("cookie", format!("session=token; token={token}"))
        .multipart(
            reqwest::multipart::Form::new()
                .text("token", token)
                .part("picture", part),
        )
        .send()
        .await
        .unwrap();
    let status = answer.status();
    let body = answer.text().await.unwrap();
    assert_eq!(status, 303, "{body}");
    assert!(
        log.calls()
            .contains(&"POST /api/topics/11111111-1111-1111-1111-111111111111/images".to_owned()),
        "{:?}",
        log.calls()
    );
}
