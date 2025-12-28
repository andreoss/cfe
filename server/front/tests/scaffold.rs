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

async fn form_token(base: &str) -> (String, String) {
    let response = http().get(base).send().await.unwrap();
    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(';').next())
        .and_then(|pair| pair.strip_prefix("token="))
        .expect("a page answers with a token")
        .to_owned();
    let page = response.text().await.unwrap();
    let token = cookie.clone();
    assert!(page.contains(&format!("value=\"{token}\"")));
    (format!("token={token}"), token)
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
    let (cookies, token) = form_token(&base).await;
    let response = http()
        .post(format!("{base}/theme"))
        .header("cookie", cookies)
        .form(&[
            ("theme", "dark"),
            ("return_to", "/sections/general"),
            ("token", &token),
        ])
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
    let (cookies, token) = form_token(&base).await;
    let response = http()
        .post(format!("{base}/theme"))
        .header("cookie", cookies)
        .form(&[("theme", "neon"), ("token", &token)])
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
    let (cookies, token) = form_token(&base).await;
    let response = http()
        .post(format!("{base}/theme"))
        .header("cookie", cookies)
        .form(&[
            ("theme", "classic"),
            ("return_to", "//elsewhere.example/"),
            ("token", &token),
        ])
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

fn heading_levels(body: &str) -> Vec<u8> {
    let mut levels = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find("<h") {
        rest = &rest[at + 2..];
        if let Some(level) = rest.chars().next().and_then(|c| c.to_digit(10)) {
            levels.push(level as u8);
        }
    }
    levels
}

fn heading_run_is_ordered(body: &str) -> bool {
    let levels = heading_levels(body);
    if levels.first() != Some(&1) {
        return false;
    }
    let mut deepest = 1u8;
    for level in levels {
        if level > deepest + 1 {
            return false;
        }
        deepest = deepest.max(level);
    }
    true
}

fn every_visible_control_is_labelled(body: &str) -> bool {
    for tag in ["input", "select", "textarea"] {
        let opener = format!("<{tag} id=\"");
        let mut rest = body;
        while let Some(at) = rest.find(&opener) {
            rest = &rest[at + opener.len()..];
            let id = &rest[..rest.find('"').unwrap_or(0)];
            if !body.contains(&format!("<label for=\"{id}\"")) {
                return false;
            }
        }
    }
    true
}

fn wayfinding_and_pager_are_named(body: &str) -> bool {
    (!body.contains("<nav class=\"ways\"") || body.contains("<nav class=\"ways\" aria-label"))
        && (!body.contains("<nav class=\"pager\"") || body.contains("<nav class=\"pager\" aria-label"))
}

const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

fn tags_are_balanced(body: &str) -> bool {
    let mut stack = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        let Some(end) = rest.find('>') else {
            return false;
        };
        let raw = &rest[..end];
        rest = &rest[end + 1..];
        if raw.starts_with('!') {
            continue;
        }
        let closing = raw.starts_with('/');
        let self_closing = raw.trim_end().ends_with('/');
        let name_part = if closing { &raw[1..] } else { raw };
        let name = name_part
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        if closing {
            match stack.pop() {
                Some(top) if top == name => {}
                _ => return false,
            }
        } else if !self_closing && !VOID_ELEMENTS.contains(&name.as_str()) {
            stack.push(name);
        }
    }
    stack.is_empty()
}

fn ids_are_unique(body: &str) -> bool {
    let opener = "id=\"";
    let mut rest = body;
    let mut seen = std::collections::BTreeSet::new();
    while let Some(at) = rest.find(opener) {
        rest = &rest[at + opener.len()..];
        let id = &rest[..rest.find('"').unwrap_or(0)];
        if !seen.insert(id.to_owned()) {
            return false;
        }
    }
    true
}

fn page_is_accessible(body: &str) -> bool {
    front::html::scripting_free(body)
        && body.matches("<h1").count() == 1
        && heading_run_is_ordered(body)
        && every_visible_control_is_labelled(body)
        && wayfinding_and_pager_are_named(body)
        && tags_are_balanced(body)
        && ids_are_unique(body)
}

#[tokio::test]
async fn every_theme_of_every_reachable_page_is_accessible() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    for theme in ["light", "classic", "dark", "contrast"] {
        for path in ["/", "/nowhere"] {
            let body = http()
                .get(format!("{base}{path}"))
                .header("cookie", format!("theme={theme}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            assert!(page_is_accessible(&body), "{theme} {path}:\n{body}");
        }
    }
}

#[tokio::test]
async fn a_page_ends_with_its_content_and_says_nothing_about_itself() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let page = http()
        .get(&base)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!page.contains("Server-rendered"));
    assert!(!page.contains("<footer"));
}

#[tokio::test]
async fn an_address_asked_for_the_wrong_method_answers_with_its_own_page() {
    let base = front(&stub("200 OK", "application/json", SECTIONS).await).await;
    let response = http().post(&base).send().await.unwrap();
    assert_eq!(response.status(), 405);
    let body = response.text().await.unwrap();
    assert!(front::html::scripting_free(&body));
    assert!(body.contains("<h1"));
    assert!(!body.is_empty());
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
