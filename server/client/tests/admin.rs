use client::{
    AddressBlock, ApiClient, ClientError, Group, Invitation, InvitationPolicy, Maintenance, Paged,
};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn last(&self) -> String {
        self.0.lock().unwrap().last().cloned().unwrap_or_default()
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

fn answer(status: &'static str, body: &'static str) -> &'static str {
    let mut response = format!("HTTP/1.1 {status}\r\n");
    response.push_str("content-type: application/json\r\n");
    response.push_str(&format!("content-length: {}\r\n", body.len()));
    response.push_str("connection: close\r\n\r\n");
    Box::leak(Box::new(response + body))
}

const SETTINGS: &str = "{\"slug\":\"news\",\"title\":\"News\",\"topics_score\":\"anyone\"}";
const GROUP: &str = concat!(
    "{\"id\":\"11111111-1111-1111-1111-111111111111\",",
    "\"section_slug\":\"general\",\"name\":\"Talk\",\"slug\":\"talk\"}"
);
const MAINTENANCE: &str = "{\"blocked\":2,\"dropped\":1}";
const POLICY: &str = "{\"required\":true}";
const INVITATIONS: &str = concat!(
    "{\"items\":[{\"code\":\"CODE-ONE\",\"expires_at\":\"2024-12-25T00:00:00Z\",",
    "\"spent\":true,\"spent_by\":\"bob\"},{\"code\":\"CODE-TWO\",",
    "\"expires_at\":\"2024-12-26T00:00:00Z\",\"spent\":false,\"spent_by\":null}],",
    "\"page\":{\"number\":1,\"size\":25,\"total\":2,\"total_pages\":1,",
    "\"has_next\":false,\"has_previous\":false}}"
);
const INVITATION: &str = concat!(
    "{\"code\":\"CODE-THREE\",\"expires_at\":\"2024-12-27T00:00:00Z\",",
    "\"spent\":false,\"spent_by\":null}"
);
const BLOCKS: &str = concat!(
    "[{\"addr\":\"203.0.113.7\",\"reason\":\"spam\",",
    "\"blocked_at\":\"2024-12-18T00:00:00Z\",\"until\":\"2025-01-17T00:00:00Z\",",
    "\"mode\":\"write\"}]"
);
const BLOCK: &str = concat!(
    "{\"addr\":\"203.0.113.9\",\"reason\":\"spam\",",
    "\"blocked_at\":\"2024-12-18T00:00:00Z\",\"until\":null,\"mode\":\"write\"}"
);

#[tokio::test]
async fn a_section_is_made_with_the_session() {
    let (base, log) = board(answer("201 Created", SETTINGS)).await;
    let client = ApiClient::new(&base).unwrap();
    let settings = client
        .create_section("token", "news", "News")
        .await
        .unwrap();
    assert_eq!(settings.slug, "news");
    assert_eq!(settings.title, "News");
    assert_eq!(settings.topics_score, "anyone");
    let request = log.last();
    assert!(request.starts_with("POST /api/sections "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"slug\":\"news\""), "{request}");
    assert!(request.contains("\"title\":\"News\""), "{request}");
}

#[tokio::test]
async fn a_section_is_renamed_with_the_session() {
    let (base, log) = board(answer("200 OK", SETTINGS)).await;
    let client = ApiClient::new(&base).unwrap();
    let settings = client
        .rename_section("token", "news", "Newses")
        .await
        .unwrap();
    assert_eq!(settings.title, "News");
    let request = log.last();
    assert!(
        request.starts_with("PATCH /api/sections/news/settings "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"title\":\"Newses\""), "{request}");
}

#[tokio::test]
async fn what_a_section_takes_to_post_is_set_with_the_session() {
    let (base, log) = board(answer("200 OK", SETTINGS)).await;
    let client = ApiClient::new(&base).unwrap();
    let settings = client
        .set_section_score("token", "general", "moderators")
        .await
        .unwrap();
    assert_eq!(settings.topics_score, "anyone");
    let request = log.last();
    assert!(
        request.starts_with("POST /api/sections/general/topics-score "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(
        request.contains("\"topics_score\":\"moderators\""),
        "{request}"
    );
}

#[tokio::test]
async fn a_group_is_made_with_the_session() {
    let (base, log) = board(answer("200 OK", GROUP)).await;
    let client = ApiClient::new(&base).unwrap();
    let group = client
        .create_group("token", "general", "Talk", "talk")
        .await
        .unwrap();
    assert_eq!(
        group,
        Group {
            id: "11111111-1111-1111-1111-111111111111".to_owned(),
            section_slug: "general".to_owned(),
            name: "Talk".to_owned(),
            slug: "talk".to_owned(),
        }
    );
    let request = log.last();
    assert!(
        request.starts_with("POST /api/sections/general/groups "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"name\":\"Talk\""), "{request}");
    assert!(request.contains("\"slug\":\"talk\""), "{request}");
}

#[tokio::test]
async fn a_group_is_renamed_with_the_session() {
    let (base, log) = board(answer("200 OK", GROUP)).await;
    let client = ApiClient::new(&base).unwrap();
    let group = client
        .rename_group("token", "general", "talk", "The Talk")
        .await
        .unwrap();
    assert_eq!(group.name, "Talk");
    let request = log.last();
    assert!(
        request.starts_with("PATCH /api/sections/general/groups/talk "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"title\":\"The Talk\""), "{request}");
}

#[tokio::test]
async fn the_maintenance_is_run_with_the_session() {
    let (base, log) = board(answer("200 OK", MAINTENANCE)).await;
    let client = ApiClient::new(&base).unwrap();
    let maintenance = client.run_maintenance("token").await.unwrap();
    assert_eq!(
        maintenance,
        Maintenance {
            blocked: 2,
            dropped: 1
        }
    );
    let request = log.last();
    assert!(
        request.starts_with("POST /api/maintenance/run "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn a_refusal_keeps_its_reason() {
    let (base, _) = board(answer("409 Conflict", "{\"error\":\"slug taken\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client
            .create_section("token", "news", "News")
            .await
            .unwrap_err(),
        ClientError::Rejected {
            status: 409,
            reason: "slug taken".to_owned()
        }
    );
}

#[tokio::test]
async fn an_answer_that_is_not_the_promised_shape_is_an_error() {
    let (base, _) = board(answer("200 OK", "{\"slug\":\"news\"}")).await;
    let client = ApiClient::new(&base).unwrap();
    assert!(matches!(
        client
            .rename_section("token", "news", "Newses")
            .await
            .unwrap_err(),
        ClientError::Detail(_)
    ));
}

#[tokio::test]
async fn the_policy_of_the_board_is_read() {
    let (base, log) = board(answer("200 OK", POLICY)).await;
    let client = ApiClient::new(&base).unwrap();
    let policy = client.invitation_policy().await.unwrap();
    assert_eq!(policy, InvitationPolicy { required: true });
    assert!(log.last().starts_with("GET /api/invitations/policy "));
}

#[tokio::test]
async fn the_invitations_of_an_account_are_read_with_the_session() {
    let (base, log) = board(answer("200 OK", INVITATIONS)).await;
    let client = ApiClient::new(&base).unwrap();
    let page: Paged<Invitation> = client.invitations("token", 1).await.unwrap();
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].code, "CODE-ONE");
    assert_eq!(page.items[0].expires_at, "2024-12-25T00:00:00Z");
    assert!(page.items[0].spent);
    assert_eq!(page.items[0].spent_by, Some("bob".to_owned()));
    assert!(!page.items[1].spent);
    assert_eq!(page.items[1].spent_by, None);
    assert_eq!(page.page.total, 2);
    let request = log.last();
    assert!(
        request.starts_with("GET /api/invitations?page=1 "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_invitation_is_issued_with_the_session() {
    let (base, log) = board(answer("201 Created", INVITATION)).await;
    let client = ApiClient::new(&base).unwrap();
    let invitation = client.issue_invitation("token").await.unwrap();
    assert_eq!(invitation.code, "CODE-THREE");
    assert!(!invitation.spent);
    let request = log.last();
    assert!(request.starts_with("POST /api/invitations "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn the_address_blocks_are_read_with_the_session() {
    let (base, log) = board(answer("200 OK", BLOCKS)).await;
    let client = ApiClient::new(&base).unwrap();
    let blocks = client.address_blocks("token").await.unwrap();
    assert_eq!(
        blocks[0],
        AddressBlock {
            addr: "203.0.113.7".to_owned(),
            reason: "spam".to_owned(),
            blocked_at: "2024-12-18T00:00:00Z".to_owned(),
            until: Some("2025-01-17T00:00:00Z".to_owned()),
            mode: "write".to_owned(),
        }
    );
    let request = log.last();
    assert!(request.starts_with("GET /api/address-blocks "), "{request}");
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_address_is_blocked_with_the_session() {
    let (base, log) = board(answer("200 OK", BLOCK)).await;
    let client = ApiClient::new(&base).unwrap();
    let block = client
        .block_address("token", "203.0.113.9", "spam", None)
        .await
        .unwrap();
    assert_eq!(block.addr, "203.0.113.9");
    assert_eq!(block.until, None);
    let request = log.last();
    assert!(
        request.starts_with("POST /api/address-blocks "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
    assert!(request.contains("\"addr\":\"203.0.113.9\""), "{request}");
    assert!(request.contains("\"reason\":\"spam\""), "{request}");
}

#[tokio::test]
async fn an_address_block_is_lifted_with_the_session() {
    let (base, log) = board(answer("204 No Content", "")).await;
    let client = ApiClient::new(&base).unwrap();
    client
        .lift_address_block("token", "203.0.113.9")
        .await
        .unwrap();
    let request = log.last();
    assert!(
        request.starts_with("DELETE /api/address-blocks/203.0.113.9 "),
        "{request}"
    );
    assert!(request.contains("cookie: session=token"), "{request}");
}

#[tokio::test]
async fn an_unspent_invitation_is_the_limit_and_an_answer_keeps_its_reason() {
    let (base, _) = board(answer(
        "422 Unprocessable Entity",
        "{\"error\":\"too many unused invitations\"}",
    ))
    .await;
    let client = ApiClient::new(&base).unwrap();
    assert_eq!(
        client.issue_invitation("token").await.unwrap_err(),
        ClientError::Rejected {
            status: 422,
            reason: "too many unused invitations".to_owned()
        }
    );
}
