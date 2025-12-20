use client::{ApiClient, ClientError, Criteria, Hit, Order, Scope};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn requests(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    fn last(&self) -> String {
        self.requests().last().cloned().unwrap_or_default()
    }
}

async fn board(response: &'static str) -> (String, Log) {
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
                sink.lock().unwrap().push(request);
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    (format!("http://{address}"), log)
}

fn answer(status: &'static str, headers: &[&'static str], body: &'static str) -> &'static str {
    let mut response = format!("HTTP/1.1 {status}\r\n");
    for header in headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    response.push_str(&format!("content-length: {}\r\n", body.len()));
    response.push_str("connection: close\r\n\r\n");
    let leaked: &'static mut String = Box::leak(Box::new(response + body));
    leaked
}

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

const NOTHING: &str = "[]";

#[tokio::test]
async fn a_search_asks_the_board_for_the_words_the_narrowing_and_the_order() {
    let (base, log) = board(answer("200 OK", &["content-type: application/json"], HITS)).await;
    let client = ApiClient::new(&base).unwrap();
    let criteria = Criteria::new("adapters", Scope::Topics, Order::Newest);
    client.search(&criteria).await.unwrap();
    let request = log.last();
    assert!(
        request.starts_with("GET /api/search?q=adapters&scope=topics&order=newest "),
        "{request}"
    );
}

#[tokio::test]
async fn a_search_is_asked_without_a_session() {
    let (base, log) = board(answer("200 OK", &["content-type: application/json"], HITS)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .search(&Criteria::new(
            "adapters",
            Scope::Everything,
            Order::Relevance,
        ))
        .await
        .unwrap();
    assert!(!log.last().contains("cookie:"), "{}", log.last());
}

#[tokio::test]
async fn the_words_stay_inside_one_step_of_the_address() {
    let (base, log) = board(answer("200 OK", &["content-type: application/json"], HITS)).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .search(&Criteria::new(
            "a/b?c d&e",
            Scope::Everything,
            Order::Relevance,
        ))
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request
            .starts_with("GET /api/search?q=a%2Fb%3Fc%20d%26e&scope=everything&order=relevance "),
        "{request}"
    );
}

#[tokio::test]
async fn a_hit_of_each_kind_comes_back_in_the_order_the_board_gave() {
    let (base, _) = board(answer("200 OK", &["content-type: application/json"], HITS)).await;
    let client = ApiClient::new(&base).unwrap();
    let hits = client
        .search(&Criteria::new(
            "adapters",
            Scope::Everything,
            Order::Relevance,
        ))
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    match &hits[0] {
        Hit::Topic(topic) => {
            assert_eq!(topic.title, "Ports and adapters");
            assert_eq!(topic.author_username, "alice");
            assert_eq!(topic.tags, vec!["rust".to_owned()]);
        }
        other => panic!("the first hit is a subject: {other:?}"),
    }
    match &hits[1] {
        Hit::Comment(remark) => {
            assert_eq!(remark.body, "Adapters keep the domain clean");
            assert_eq!(remark.author_username, "bob");
            assert_eq!(
                remark.topic_id,
                "11111111-1111-1111-1111-111111111111".to_owned()
            );
        }
        other => panic!("the second hit is a remark: {other:?}"),
    }
}

#[tokio::test]
async fn a_search_nobody_wrote_anything_for_comes_back_empty() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        NOTHING,
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    let hits = client
        .search(&Criteria::new("nothing", Scope::Comments, Order::Oldest))
        .await
        .unwrap();
    assert!(hits.is_empty());
}

#[tokio::test]
async fn a_search_the_board_refuses_keeps_its_reason() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        &["content-type: application/json"],
        "{\"error\":\"invalid query\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client
        .search(&Criteria::new("a", Scope::Everything, Order::Relevance))
        .await
    {
        Err(ClientError::Rejected { status, reason }) => {
            assert_eq!(status, 422);
            assert_eq!(reason, "invalid query");
        }
        other => panic!("a refused search must keep its reason: {other:?}"),
    }
}

#[tokio::test]
async fn an_unreadable_list_of_hits_is_an_error() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        "not json",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client
        .search(&Criteria::new(
            "adapters",
            Scope::Everything,
            Order::Relevance,
        ))
        .await
    {
        Err(ClientError::Detail(_)) => {}
        other => panic!("an unreadable answer must be an error: {other:?}"),
    }
}

#[tokio::test]
async fn a_hit_of_a_kind_nobody_knows_is_an_error() {
    let (base, _) = board(answer(
        "200 OK",
        &["content-type: application/json"],
        "[{\"kind\":\"section\"}]",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    match client
        .search(&Criteria::new(
            "adapters",
            Scope::Everything,
            Order::Relevance,
        ))
        .await
    {
        Err(ClientError::Detail(_)) => {}
        other => panic!("a hit of an unknown kind must be an error: {other:?}"),
    }
}

#[test]
fn every_narrowing_and_order_is_read_back_by_its_word() {
    for scope in Scope::ALL {
        assert_eq!(Scope::parse(scope.label()), Some(scope));
    }
    for order in Order::ALL {
        assert_eq!(Order::parse(order.label()), Some(order));
    }
}

#[test]
fn a_narrowing_or_order_it_does_not_know_falls_back_to_the_wide_one() {
    let criteria = Criteria::parse("adapters", Some("everywhere"), Some("loudest"));
    assert_eq!(criteria.scope(), Scope::Everything);
    assert_eq!(criteria.order(), Order::Relevance);
    assert_eq!(criteria.query(), "adapters");
    assert_eq!(
        Criteria::parse("adapters", None, None),
        Criteria::new("adapters", Scope::Everything, Order::Relevance)
    );
}

#[test]
fn a_hit_keeps_the_kind_it_came_in_as() {
    let topic = client::Topic {
        id: "1".to_owned(),
        section_slug: "general".to_owned(),
        title: "Ports".to_owned(),
        author_username: "alice".to_owned(),
        created_at: "2024-06-07T10:11:12Z".to_owned(),
        tags: Vec::new(),
        sticky: false,
        resolved: false,
        deleted: false,
        pending: false,
        draft: false,
        postscore: 0,
    };
    assert_eq!(Hit::Topic(topic.clone()).kind(), "topic");
    assert_eq!(
        Hit::Topic(topic.clone())
            .subject()
            .map(|t| t.title.as_str()),
        Some("Ports")
    );
    assert!(Hit::Topic(topic).remark().is_none());
}
