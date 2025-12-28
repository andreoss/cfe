use front::config::Config;
use front::routes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SECTIONS: &str = "[{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"anyone\",\"may_post\":true}]";

const THEMES: [&str; 4] = ["classic", "dark", "contrast", "light"];

const PAGES: [&str; 7] = [
    "/",
    "/sign-in",
    "/register",
    "/forgot",
    "/activate",
    "/search",
    "/nowhere",
];

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

async fn stub() -> String {
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
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    SECTIONS.len(),
                    SECTIONS
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
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
async fn a_chosen_theme_is_on_the_html_and_the_body_of_every_page() {
    let base = front(&stub().await).await;
    for theme in THEMES {
        for path in PAGES {
            let page = http()
                .get(format!("{base}{path}"))
                .header("cookie", format!("theme={theme}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            let html = page
                .lines()
                .find(|line| line.starts_with("<html"))
                .expect("a page opens its html element");
            assert!(
                html.contains(&format!("class=\"theme-{theme}\"")),
                "{path} in {theme} misses the theme on the html"
            );
            assert!(
                html.contains("lang=\"en\""),
                "{path} in {theme} carries no language"
            );
            let body = page
                .lines()
                .find(|line| line.trim_start().starts_with("<body"))
                .expect("a page opens its body element");
            assert!(
                body.contains(&format!("class=\"theme-{theme}\"")),
                "{path} in {theme} misses the theme on the body"
            );
        }
    }
}

#[tokio::test]
async fn a_reader_with_no_choice_keeps_the_light_theme() {
    let base = front(&stub().await).await;
    let page = http()
        .get(&base)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        page.lines()
            .any(|line| line.starts_with("<html") && line.contains("class=\"theme-light\""))
    );
}

#[tokio::test]
async fn the_stylesheet_carries_the_four_themes_and_they_differ() {
    let base = front(&stub().await).await;
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
    let css = response.text().await.unwrap();
    for theme in ["classic", "dark", "contrast"] {
        assert!(
            css.contains(&format!("body.theme-{theme}")),
            "the stylesheet lacks the {theme} theme"
        );
    }
    let mut pages: Vec<String> = css
        .lines()
        .filter_map(|line| line.split("--page:").nth(1))
        .map(|value| value.trim().to_owned())
        .collect();
    pages.sort();
    pages.dedup();
    assert!(pages.len() >= 3, "the themes look alike: {pages:?}");
}

#[tokio::test]
async fn a_theme_choice_without_the_token_of_the_page_is_refused() {
    let base = front(&stub().await).await;
    let response = http()
        .post(format!("{base}/theme"))
        .form(&[("theme", "dark")])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    let cookies = response.headers().get_all("set-cookie");
    let theme_set = cookies.iter().any(|value| {
        value
            .to_str()
            .map(|raw| raw.split(';').next().unwrap_or("").starts_with("theme="))
            .unwrap_or(false)
    });
    assert!(!theme_set, "a refused choice changes the theme");
}
