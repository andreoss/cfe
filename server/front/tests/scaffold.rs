use front::config::Config;
use front::routes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECTIONS: &str = "[{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"anyone\",\"may_post\":true},{\"slug\":\"news\",\"title\":\"News\",\"topics_score\":\"moderators\",\"may_post\":false}]";

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

async fn stub(status: &'static str, content_type: &'static str, body: &'static str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(_) => return,
            };
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 4096];
                let _ = socket.read(&mut buffer).await;
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    format!("http://{address}")
}

async fn closed_address() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}")
}

async fn front(server_url: &str) -> String {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(front::config::SERVER_URL.to_owned(), server_url.to_owned());
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
async fn the_health_route_says_it_is_there() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http().get(format!("{base}/healthz")).send().await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn the_section_list_comes_from_the_api() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let page = http()
        .get(&base)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("href=\"/sections/general\""));
    assert!(page.contains("General Talk"));
    assert!(page.contains("href=\"/sections/news\""));
    assert!(page.contains("News"));
}

#[tokio::test]
async fn a_rendered_page_carries_no_scripting() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    for path in ["/", "/healthz", "/static/style.css", "/nowhere"] {
        let body = http()
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(
            front::html::scripting_free(&body),
            "{path} carries scripting"
        );
        assert!(!body.to_ascii_lowercase().contains("<script"), "{path}");
    }
}

#[tokio::test]
async fn the_stylesheet_is_served_as_css() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http()
        .get(format!("{base}/static/style.css"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/css")
    );
    assert!(response.text().await.unwrap().contains("body.theme-dark"));
}

#[tokio::test]
async fn a_theme_choice_sets_a_cookie_and_returns_to_the_page() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http()
        .post(format!("{base}/theme"))
        .form(&[("theme", "dark"), ("return_to", "/sections/general")])
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
        "/sections/general"
    );
    let cookie = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(cookie.starts_with("theme=dark"), "{cookie}");
}

#[tokio::test]
async fn the_chosen_theme_is_applied_to_the_next_page() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let page = http()
        .get(&base)
        .header("cookie", "theme=contrast")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(page.contains("class=\"theme-contrast\""));
}

#[tokio::test]
async fn an_unknown_theme_is_refused_without_changing_anything() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http()
        .post(format!("{base}/theme"))
        .form(&[("theme", "neon")])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    assert!(response.headers().get("set-cookie").is_none());
    assert!(response.text().await.unwrap().contains("No such theme"));
}

#[tokio::test]
async fn a_return_address_that_leaves_the_site_is_ignored() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http()
        .post(format!("{base}/theme"))
        .form(&[("theme", "classic"), ("return_to", "//elsewhere.example/")])
        .send()
        .await
        .unwrap();
    assert_eq!(
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/"
    );
}

#[tokio::test]
async fn a_board_that_does_not_answer_gives_a_page_and_not_a_crash() {
    let base = front(&closed_address().await).await;
    let response = http().get(&base).send().await.unwrap();
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
async fn a_board_that_answers_badly_gives_a_page_too() {
    let base = front(&stub("500 Internal Server Error", "text/plain", "no").await).await;
    let response = http().get(&base).send().await.unwrap();
    assert_eq!(response.status(), 503);
}

#[tokio::test]
async fn a_page_that_does_not_exist_is_named_as_such() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http()
        .get(format!("{base}/nothing/here"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert!(response.text().await.unwrap().contains("Nothing here"));
}
