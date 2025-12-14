use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Section {
    pub slug: String,
    pub title: String,
    pub topics_score: String,
    pub may_post: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Topic {
    pub id: String,
    pub section_slug: String,
    pub title: String,
    pub author_username: String,
    pub created_at: String,
    pub tags: Vec<String>,
    pub sticky: bool,
    pub resolved: bool,
    pub deleted: bool,
    pub pending: bool,
    pub draft: bool,
    pub postscore: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Subject {
    pub id: String,
    pub section_slug: String,
    pub group_slug: Option<String>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub author_username: String,
    pub created_at: String,
    pub deleted: bool,
    pub deleted_reason: Option<String>,
    pub edited: bool,
    pub postscore: i32,
    pub pending: bool,
    pub draft: bool,
    pub sticky: bool,
    pub off_front: bool,
    pub resolved: bool,
    pub minor: bool,
    pub open_reports: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Comment {
    pub id: String,
    pub topic_id: String,
    pub parent_id: Option<String>,
    pub body: String,
    pub author_username: String,
    pub created_at: String,
    pub deleted: bool,
    pub deleted_reason: Option<String>,
    pub edited: bool,
    pub ignored: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub comment: Comment,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PageInfo {
    pub number: u32,
    pub size: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_previous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Paged<T> {
    pub items: Vec<T>,
    pub page: PageInfo,
}

pub fn find_section<'a>(sections: &'a [Section], slug: &str) -> Option<&'a Section> {
    sections.iter().find(|section| section.slug == slug)
}

pub fn thread(comments: &[Comment]) -> Vec<Branch> {
    let known: BTreeMap<&str, &Comment> = comments
        .iter()
        .map(|comment| (comment.id.as_str(), comment))
        .collect();
    let mut replies: BTreeMap<&str, Vec<&Comment>> = BTreeMap::new();
    let mut roots: Vec<&Comment> = Vec::new();
    for comment in comments {
        match comment.parent_id.as_deref() {
            Some(parent) if parent != comment.id.as_str() && known.contains_key(parent) => {
                replies.entry(parent).or_default().push(comment)
            }
            _ => roots.push(comment),
        }
    }
    let by_age = |left: &&Comment, right: &&Comment| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    };
    for siblings in replies.values_mut() {
        siblings.sort_by(by_age);
    }
    roots.sort_by(by_age);
    let mut out = Vec::with_capacity(comments.len());
    let mut stack: Vec<(&Comment, usize)> = roots.into_iter().rev().map(|c| (c, 0)).collect();
    while let Some((comment, depth)) = stack.pop() {
        out.push(Branch {
            comment: comment.clone(),
            depth,
        });
        if let Some(children) = replies.get(comment.id.as_str()) {
            stack.extend(children.iter().rev().map(|child| (*child, depth + 1)));
        }
    }
    out
}

pub fn encode_path(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClientError {
    BadBaseUrl(String),
    Transport(String),
    Status(u16),
    Detail(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::BadBaseUrl(raw) => {
                write!(f, "the server address is not an http address: {raw}")
            }
            ClientError::Transport(detail) => {
                write!(f, "the server could not be reached: {detail}")
            }
            ClientError::Status(status) => write!(f, "the server answered {status}"),
            ClientError::Detail(detail) => {
                write!(f, "the server sent an unreadable answer: {detail}")
            }
        }
    }
}

impl std::error::Error for ClientError {}

#[derive(Clone)]
pub struct ApiClient {
    base: String,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base: &str) -> Result<Self, ClientError> {
        let trimmed = base.trim_end_matches('/').to_owned();
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            return Err(ClientError::BadBaseUrl(base.to_owned()));
        }
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(Self {
            base: trimmed,
            http,
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub async fn sections(&self) -> Result<Vec<Section>, ClientError> {
        self.get("/api/sections").await
    }

    pub async fn topics(&self, slug: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.get(&format!(
            "/api/sections/{}/topics?page={}",
            encode_path(slug),
            page
        ))
        .await
    }

    pub async fn subject(&self, id: &str) -> Result<Subject, ClientError> {
        self.get(&format!("/api/topics/{}", encode_path(id))).await
    }

    pub async fn comments(&self, id: &str, page: u32) -> Result<Paged<Comment>, ClientError> {
        self.get(&format!(
            "/api/topics/{}/comments?page={}",
            encode_path(id),
            page
        ))
        .await
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ClientError> {
        let response = self
            .http
            .get(format!("{}{}", self.base, path))
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(ClientError::Status(status));
        }
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_http_address_is_accepted_and_kept_without_a_trailing_slash() {
        let client = ApiClient::new("http://127.0.0.1:8080/").unwrap();
        assert_eq!(client.base(), "http://127.0.0.1:8080");
    }

    #[test]
    fn an_address_that_is_not_http_is_refused() {
        match ApiClient::new("postgres://localhost/tcbs") {
            Err(ClientError::BadBaseUrl(raw)) => {
                assert_eq!(raw, "postgres://localhost/tcbs".to_owned())
            }
            _ => panic!("an address that is not http must be refused"),
        }
    }

    #[test]
    fn an_empty_address_is_refused() {
        assert!(ApiClient::new("").is_err());
    }

    #[test]
    fn an_odd_slug_stays_inside_one_step_of_the_address() {
        assert_eq!(encode_path("general"), "general");
        assert_eq!(encode_path("a/b"), "a%2Fb");
        assert_eq!(encode_path("a?b c"), "a%3Fb%20c");
        assert_eq!(encode_path("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(encode_path(""), "");
    }

    #[test]
    fn a_section_is_found_by_its_address_only() {
        let sections = vec![
            Section {
                slug: "general".to_owned(),
                title: "General Talk".to_owned(),
                topics_score: "anyone".to_owned(),
                may_post: true,
            },
            Section {
                slug: "news".to_owned(),
                title: "News".to_owned(),
                topics_score: "moderators".to_owned(),
                may_post: false,
            },
        ];
        assert_eq!(
            find_section(&sections, "news").map(|s| s.slug.as_str()),
            Some("news")
        );
        assert!(find_section(&sections, "general-2").is_none());
        assert!(find_section(&[], "general").is_none());
    }

    #[test]
    fn errors_are_described_in_words() {
        assert_eq!(
            ClientError::Status(503).to_string(),
            "the server answered 503"
        );
        assert_eq!(
            ClientError::BadBaseUrl("ftp://x".to_owned()).to_string(),
            "the server address is not an http address: ftp://x"
        );
        assert!(
            ClientError::Transport("timeout".to_owned())
                .to_string()
                .contains("timeout")
        );
        assert!(
            ClientError::Detail("bad json".to_owned())
                .to_string()
                .contains("bad json")
        );
    }
}
