use client::{ApiClient, ClientError, Section};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

async fn closed_port() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{address}")
}

#[tokio::test]
async fn the_section_list_is_read_from_the_api() {
    let base = stub("200 OK", "application/json", "[{\"slug\":\"general\",\"title\":\"General Talk\",\"topics_score\":\"anyone\",\"may_post\":true}]").await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.sections().await.unwrap(),
        vec![Section {
            slug: "general".to_owned(),
            title: "General Talk".to_owned(),
            topics_score: "anyone".to_owned(),
            may_post: true,
        }]
    );
}

#[tokio::test]
async fn an_empty_list_is_an_empty_page() {
    let base = stub("200 OK", "application/json", "[]").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(client.sections().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_status_outside_the_success_range_is_an_error() {
    let base = stub("503 Service Unavailable", "application/json", "{}").await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.sections().await.unwrap_err(),
        ClientError::Status(503)
    );
}

#[tokio::test]
async fn an_answer_that_is_not_the_promised_shape_is_an_error() {
    let base = stub("200 OK", "application/json", "{\"slug\":\"general\"}").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.sections().await.unwrap_err(),
        ClientError::Detail(_)
    ));
}

#[tokio::test]
async fn an_answer_that_is_not_json_is_an_error() {
    let base = stub("200 OK", "text/html", "<html>nope</html>").await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client.sections().await.unwrap_err(),
        ClientError::Detail(_)
    ));
}

#[tokio::test]
async fn a_server_that_is_not_there_is_reported_not_raised() {
    let client = ApiClient::new(&closed_port().await).unwrap();
    assert!(matches!(
        client.sections().await.unwrap_err(),
        ClientError::Transport(_)
    ));
}
