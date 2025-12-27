use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SESSION_COOKIE: &str = "session";

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Version {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub editor: String,
    pub written_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Change {
    pub kind: String,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewSubject {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub group: Option<String>,
    pub challenge: Option<String>,
    pub draft: bool,
}

impl NewSubject {
    pub fn new(title: &str, body: &str) -> Self {
        Self {
            title: title.to_owned(),
            body: body.to_owned(),
            tags: Vec::new(),
            group: None,
            challenge: None,
            draft: false,
        }
    }

    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        self
    }

    pub fn group(mut self, slug: &str) -> Self {
        self.group = Some(slug.to_owned());
        self
    }

    pub fn challenge(mut self, answer: &str) -> Self {
        self.challenge = Some(answer.to_owned());
        self
    }

    pub fn draft(mut self) -> Self {
        self.draft = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewRemark {
    pub body: String,
    pub parent_id: Option<String>,
    pub challenge: Option<String>,
}

impl NewRemark {
    pub fn new(body: &str) -> Self {
        Self {
            body: body.to_owned(),
            parent_id: None,
            challenge: None,
        }
    }

    pub fn reply_to(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_owned());
        self
    }

    pub fn challenge(mut self, answer: &str) -> Self {
        self.challenge = Some(answer.to_owned());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubjectEdit {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub minor: bool,
}

impl SubjectEdit {
    pub fn new(title: &str, body: &str) -> Self {
        Self {
            title: title.to_owned(),
            body: body.to_owned(),
            tags: Vec::new(),
            minor: false,
        }
    }

    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        self
    }

    pub fn minor(mut self) -> Self {
        self.minor = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Removal {
    pub reason: String,
    pub penalty: Option<i32>,
}

impl Removal {
    pub fn new(reason: &str) -> Self {
        Self {
            reason: reason.to_owned(),
            penalty: None,
        }
    }

    pub fn penalty(mut self, penalty: i32) -> Self {
        self.penalty = Some(penalty);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Hit {
    Topic(Topic),
    Comment(Comment),
}

impl Hit {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Topic(_) => "topic",
            Self::Comment(_) => "comment",
        }
    }

    pub fn subject(&self) -> Option<&Topic> {
        match self {
            Self::Topic(topic) => Some(topic),
            Self::Comment(_) => None,
        }
    }

    pub fn remark(&self) -> Option<&Comment> {
        match self {
            Self::Topic(_) => None,
            Self::Comment(remark) => Some(remark),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Everything,
    Topics,
    Comments,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Self::Everything, Self::Topics, Self::Comments];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "everything" => Some(Self::Everything),
            "topics" => Some(Self::Topics),
            "comments" => Some(Self::Comments),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Everything => "everything",
            Self::Topics => "topics",
            Self::Comments => "comments",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::Everything => "Everything",
            Self::Topics => "Subjects",
            Self::Comments => "Remarks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Relevance,
    Newest,
    Oldest,
}

impl Order {
    pub const ALL: [Order; 3] = [Self::Relevance, Self::Newest, Self::Oldest];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "relevance" => Some(Self::Relevance),
            "newest" => Some(Self::Newest),
            "oldest" => Some(Self::Oldest),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Newest => "newest",
            Self::Oldest => "oldest",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::Relevance => "Best fit",
            Self::Newest => "Newest first",
            Self::Oldest => "Oldest first",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criteria {
    query: String,
    scope: Scope,
    order: Order,
}

impl Criteria {
    pub fn new(query: &str, scope: Scope, order: Order) -> Self {
        Self {
            query: query.to_owned(),
            scope,
            order,
        }
    }

    pub fn parse(query: &str, scope: Option<&str>, order: Option<&str>) -> Self {
        Self::new(
            query,
            scope.and_then(Scope::parse).unwrap_or(Scope::Everything),
            order.and_then(Order::parse).unwrap_or(Order::Relevance),
        )
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn scope(&self) -> Scope {
        self.scope
    }

    pub fn order(&self) -> Order {
        self.order
    }
}

pub fn search_address(criteria: &Criteria) -> String {
    format!(
        "/api/search?q={}&scope={}&order={}",
        encode_path(criteria.query()),
        criteria.scope().label(),
        criteria.order().label()
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Notification {
    pub id: String,
    pub topic_id: String,
    pub topic_title: String,
    pub comment_id: String,
    pub actor_username: String,
    pub created_at: String,
    pub read: bool,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UnreadCount {
    pub unread: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ArchiveMonth {
    pub year: i32,
    pub month: u8,
    pub topics: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tag {
    pub slug: String,
    pub description: Option<String>,
    pub means: Option<String>,
    pub following: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Profile {
    pub id: String,
    pub username: String,
    pub bio: Option<String>,
    pub score: i32,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    content_type: String,
    body: String,
}

impl Feed {
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avatar {
    content_type: String,
    bytes: Vec<u8>,
}

impl Avatar {
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    content_type: String,
    bytes: Vec<u8>,
}

impl Picture {
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Image {
    pub id: String,
    pub content_type: String,
    pub uploaded_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Group {
    pub id: String,
    pub section_slug: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PollOption {
    pub id: String,
    pub text: String,
    pub votes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Poll {
    pub id: String,
    pub topic_id: String,
    pub question: String,
    pub options: Vec<PollOption>,
    pub mine: Option<String>,
    pub total_votes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReactionCount {
    pub kind: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Reactions {
    pub counts: Vec<ReactionCount>,
    pub mine: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    token: String,
    cookie: String,
}

impl Session {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn cookie(&self) -> &str {
        &self.cookie
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RegisterBody {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitation: Option<String>,
}

impl RegisterBody {
    pub fn new(username: &str, email: &str, password: &str) -> Self {
        Self {
            username: username.to_owned(),
            email: email.to_owned(),
            password: password.to_owned(),
            challenge: None,
            invitation: None,
        }
    }

    pub fn invitation(mut self, code: &str) -> Self {
        self.invitation = Some(code.to_owned());
        self
    }

    pub fn challenge(mut self, answer: &str) -> Self {
        self.challenge = Some(answer.to_owned());
        self
    }
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
    Rejected { status: u16, reason: String },
    Detail(String),
}

impl ClientError {
    pub fn status(&self) -> Option<u16> {
        match self {
            ClientError::Status(status) | ClientError::Rejected { status, .. } => Some(*status),
            _ => None,
        }
    }
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
            ClientError::Rejected { reason, .. } => write!(f, "{reason}"),
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
        self.get("/api/sections", None).await
    }

    pub async fn topics(&self, slug: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.get(
            &format!("/api/sections/{}/topics?page={}", encode_path(slug), page),
            None,
        )
        .await
    }

    pub async fn subject(&self, id: &str, session: Option<&str>) -> Result<Subject, ClientError> {
        self.get(&format!("/api/topics/{}", encode_path(id)), session)
            .await
    }

    pub async fn comments(
        &self,
        id: &str,
        page: u32,
        session: Option<&str>,
    ) -> Result<Paged<Comment>, ClientError> {
        self.get(
            &format!("/api/topics/{}/comments?page={}", encode_path(id), page),
            session,
        )
        .await
    }

    pub async fn create_topic(
        &self,
        session: &str,
        slug: &str,
        body: &NewSubject,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/sections/{}/topics", encode_path(slug));
        json_of(self.post(&path, body, Some(session)).await?).await
    }

    pub async fn create_comment(
        &self,
        session: &str,
        topic_id: &str,
        body: &NewRemark,
    ) -> Result<Comment, ClientError> {
        let path = format!("/api/topics/{}/comments", encode_path(topic_id));
        json_of(self.post(&path, body, Some(session)).await?).await
    }

    pub async fn edit_topic(
        &self,
        session: &str,
        id: &str,
        body: &SubjectEdit,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}", encode_path(id));
        json_of(self.patch(&path, body, Some(session)).await?).await
    }

    pub async fn edit_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
        body: &str,
    ) -> Result<Comment, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}",
            encode_path(topic_id),
            encode_path(id)
        );
        json_of(
            self.patch(&path, &RemarkEdit { body }, Some(session))
                .await?,
        )
        .await
    }

    pub async fn delete_topic(
        &self,
        session: &str,
        id: &str,
        body: &Removal,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/delete", encode_path(id));
        json_of(self.post(&path, body, Some(session)).await?).await
    }

    pub async fn restore_topic(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/restore", encode_path(id));
        json_of(self.post(&path, &Nothing {}, Some(session)).await?).await
    }

    pub async fn delete_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
        body: &Removal,
    ) -> Result<Comment, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}/delete",
            encode_path(topic_id),
            encode_path(id)
        );
        json_of(self.post(&path, body, Some(session)).await?).await
    }

    pub async fn restore_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
    ) -> Result<Comment, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}/restore",
            encode_path(topic_id),
            encode_path(id)
        );
        json_of(self.post(&path, &Nothing {}, Some(session)).await?).await
    }

    pub async fn publish(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/publish", encode_path(id));
        json_of(self.post(&path, &Nothing {}, Some(session)).await?).await
    }

    pub async fn commit(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/commit", encode_path(id));
        json_of(self.post(&path, &Nothing {}, Some(session)).await?).await
    }

    pub async fn uncommit(&self, session: &str, id: &str) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/uncommit", encode_path(id));
        json_of(self.post(&path, &Nothing {}, Some(session)).await?).await
    }

    pub async fn sticky(
        &self,
        session: &str,
        id: &str,
        pinned: bool,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/sticky", encode_path(id));
        json_of(
            self.post(&path, &StickyBody { sticky: pinned }, Some(session))
                .await?,
        )
        .await
    }

    pub async fn off_front(
        &self,
        session: &str,
        id: &str,
        hidden: bool,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/off-front", encode_path(id));
        json_of(
            self.post(&path, &OffFrontBody { off_front: hidden }, Some(session))
                .await?,
        )
        .await
    }

    pub async fn resolved(
        &self,
        session: &str,
        id: &str,
        done: bool,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/resolved", encode_path(id));
        json_of(
            self.post(&path, &ResolvedBody { resolved: done }, Some(session))
                .await?,
        )
        .await
    }

    pub async fn postscore(
        &self,
        session: &str,
        id: &str,
        score: i32,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/postscore", encode_path(id));
        json_of(
            self.post(&path, &ScoreBody { postscore: score }, Some(session))
                .await?,
        )
        .await
    }

    pub async fn move_to(
        &self,
        session: &str,
        id: &str,
        group: &str,
    ) -> Result<Subject, ClientError> {
        let path = format!("/api/topics/{}/move", encode_path(id));
        json_of(self.post(&path, &MoveBody { group }, Some(session)).await?).await
    }

    pub async fn groups(&self, slug: &str) -> Result<Vec<Group>, ClientError> {
        self.get(&format!("/api/sections/{}/groups", encode_path(slug)), None)
            .await
    }

    pub async fn images(&self, id: &str) -> Result<Vec<Image>, ClientError> {
        self.get(&format!("/api/topics/{}/images", encode_path(id)), None)
            .await
    }

    pub async fn image(
        &self,
        topic_id: &str,
        image_id: &str,
    ) -> Result<Option<Picture>, ClientError> {
        let path = format!(
            "/api/topics/{}/images/{}",
            encode_path(topic_id),
            encode_path(image_id)
        );
        self.bytes(&path).await
    }

    pub async fn attach_image(
        &self,
        session: &str,
        id: &str,
        data: &str,
    ) -> Result<Image, ClientError> {
        let path = format!("/api/topics/{}/images", encode_path(id));
        json_of(self.post(&path, &ImageBody { data }, Some(session)).await?).await
    }

    pub async fn remove_image(
        &self,
        session: &str,
        topic_id: &str,
        image_id: &str,
    ) -> Result<(), ClientError> {
        let path = format!(
            "/api/topics/{}/images/{}",
            encode_path(topic_id),
            encode_path(image_id)
        );
        self.delete(&path, Some(session)).await?;
        Ok(())
    }

    pub async fn poll(&self, session: Option<&str>, id: &str) -> Result<Option<Poll>, ClientError> {
        self.maybe_get(&format!("/api/topics/{}/poll", encode_path(id)), session)
            .await
    }

    pub async fn create_poll(
        &self,
        session: &str,
        id: &str,
        question: &str,
        options: &[&str],
    ) -> Result<Poll, ClientError> {
        let path = format!("/api/topics/{}/poll", encode_path(id));
        let body = PollBody {
            question,
            options: options.iter().map(|text| (*text).to_owned()).collect(),
        };
        json_of(self.post(&path, &body, Some(session)).await?).await
    }

    pub async fn vote(
        &self,
        session: &str,
        id: &str,
        option_id: &str,
    ) -> Result<Poll, ClientError> {
        let path = format!("/api/topics/{}/poll/vote", encode_path(id));
        json_of(
            self.post(&path, &VoteBody { option_id }, Some(session))
                .await?,
        )
        .await
    }

    pub async fn topic_reactions(
        &self,
        session: Option<&str>,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        self.get(
            &format!("/api/topics/{}/reactions", encode_path(id)),
            session,
        )
        .await
    }

    pub async fn comment_reactions(
        &self,
        session: Option<&str>,
        topic_id: &str,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}/reactions",
            encode_path(topic_id),
            encode_path(id)
        );
        self.get(&path, session).await
    }

    pub async fn react_to_topic(
        &self,
        session: &str,
        id: &str,
        kind: &str,
    ) -> Result<Reactions, ClientError> {
        let path = format!("/api/topics/{}/reactions", encode_path(id));
        json_of(self.post(&path, &ReactBody { kind }, Some(session)).await?).await
    }

    pub async fn react_to_comment(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
        kind: &str,
    ) -> Result<Reactions, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}/reactions",
            encode_path(topic_id),
            encode_path(id)
        );
        json_of(self.post(&path, &ReactBody { kind }, Some(session)).await?).await
    }

    pub async fn clear_topic_reaction(
        &self,
        session: &str,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        let path = format!("/api/topics/{}/reactions", encode_path(id));
        json_of(self.delete(&path, Some(session)).await?).await
    }

    pub async fn clear_comment_reaction(
        &self,
        session: &str,
        topic_id: &str,
        id: &str,
    ) -> Result<Reactions, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}/reactions",
            encode_path(topic_id),
            encode_path(id)
        );
        json_of(self.delete(&path, Some(session)).await?).await
    }

    pub async fn topic_history(&self, id: &str) -> Result<Vec<Version>, ClientError> {
        self.get(&format!("/api/topics/{}/history", encode_path(id)), None)
            .await
    }

    pub async fn comment_history(
        &self,
        topic_id: &str,
        id: &str,
    ) -> Result<Vec<Version>, ClientError> {
        let path = format!(
            "/api/topics/{}/comments/{}/history",
            encode_path(topic_id),
            encode_path(id)
        );
        self.get(&path, None).await
    }

    pub async fn topic_difference(
        &self,
        id: &str,
        version_id: &str,
    ) -> Result<Vec<Change>, ClientError> {
        let path = format!(
            "/api/topics/{}/history/{}",
            encode_path(id),
            encode_path(version_id)
        );
        self.get(&path, None).await
    }

    pub async fn search(&self, criteria: &Criteria) -> Result<Vec<Hit>, ClientError> {
        self.get(&search_address(criteria), None).await
    }

    pub async fn archive(&self, session: Option<&str>) -> Result<Vec<ArchiveMonth>, ClientError> {
        self.get("/api/archive", session).await
    }

    pub async fn archive_month(
        &self,
        year: i32,
        month: u8,
        page: u32,
        session: Option<&str>,
    ) -> Result<Paged<Topic>, ClientError> {
        self.get(&format!("/api/archive/{year}/{month}?page={page}"), session)
            .await
    }

    pub async fn activity(&self, session: Option<&str>) -> Result<Vec<Hit>, ClientError> {
        self.get("/api/activity", session).await
    }

    pub async fn bookmarks(&self, session: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.get(&format!("/api/bookmarks?page={page}"), Some(session))
            .await
    }

    pub async fn watched(&self, session: &str, page: u32) -> Result<Paged<Topic>, ClientError> {
        self.get(&format!("/api/watched?page={page}"), Some(session))
            .await
    }

    pub async fn followed_tags(&self, session: &str) -> Result<Vec<String>, ClientError> {
        self.get("/api/followed-tags", Some(session)).await
    }

    pub async fn notifications(
        &self,
        session: &str,
        page: u32,
    ) -> Result<Paged<Notification>, ClientError> {
        self.get(&format!("/api/notifications?page={page}"), Some(session))
            .await
    }

    pub async fn unread_count(&self, session: &str) -> Result<UnreadCount, ClientError> {
        self.get("/api/notifications/unread-count", Some(session))
            .await
    }

    pub async fn mark_read(&self, session: &str, id: &str) -> Result<(), ClientError> {
        let path = format!("/api/notifications/{}/read", encode_path(id));
        self.post(&path, &Nothing {}, Some(session)).await?;
        Ok(())
    }

    pub async fn tag(&self, name: &str, session: Option<&str>) -> Result<Tag, ClientError> {
        self.get(&format!("/api/tags/{}", encode_path(name)), session)
            .await
    }

    pub async fn tag_topics(
        &self,
        name: &str,
        page: u32,
        session: Option<&str>,
    ) -> Result<Paged<Topic>, ClientError> {
        self.get(
            &format!("/api/tags/{}/topics?page={}", encode_path(name), page),
            session,
        )
        .await
    }

    pub async fn follow_tag(&self, session: &str, name: &str) -> Result<(), ClientError> {
        let path = format!("/api/tags/{}/follow", encode_path(name));
        self.post(&path, &Nothing {}, Some(session)).await?;
        Ok(())
    }

    pub async fn unfollow_tag(&self, session: &str, name: &str) -> Result<(), ClientError> {
        let path = format!("/api/tags/{}/follow", encode_path(name));
        self.delete(&path, Some(session)).await?;
        Ok(())
    }

    pub async fn describe_tag(
        &self,
        session: &str,
        name: &str,
        words: &str,
    ) -> Result<Tag, ClientError> {
        let body = DescribeBody { description: words };
        let response = self
            .patch(
                &format!("/api/tags/{}", encode_path(name)),
                &body,
                Some(session),
            )
            .await?;
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    pub async fn section_feed(&self, slug: &str) -> Result<Option<Feed>, ClientError> {
        self.feed(&format!("/api/sections/{}/feed", encode_path(slug)))
            .await
    }

    pub async fn tag_feed(&self, name: &str) -> Result<Option<Feed>, ClientError> {
        self.feed(&format!("/api/tags/{}/feed", encode_path(name)))
            .await
    }

    pub async fn say_means(
        &self,
        session: &str,
        name: &str,
        means: &str,
    ) -> Result<Tag, ClientError> {
        let body = MeansBody { means };
        let response = self
            .post(
                &format!("/api/tags/{}/means", encode_path(name)),
                &body,
                Some(session),
            )
            .await?;
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    pub async fn me(&self, session: Option<&str>) -> Result<Option<User>, ClientError> {
        self.get("/api/me", session).await
    }

    pub async fn profile(&self, username: &str) -> Result<Profile, ClientError> {
        self.get(&format!("/api/users/{}", encode_path(username)), None)
            .await
    }

    pub async fn update_bio(
        &self,
        session: &str,
        bio: Option<&str>,
    ) -> Result<Profile, ClientError> {
        let body = BioBody { bio };
        let response = self.patch("/api/me/bio", &body, Some(session)).await?;
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    pub async fn change_password(
        &self,
        session: &str,
        current: &str,
        new: &str,
    ) -> Result<Option<String>, ClientError> {
        let body = ChangePasswordBody {
            current_password: current,
            new_password: new,
        };
        let response = self.post("/api/me/password", &body, Some(session)).await?;
        Ok(session_cookie_of(&response))
    }

    pub async fn request_reset(&self, email: &str) -> Result<(), ClientError> {
        let body = EmailBody { email };
        self.post("/api/password-reset", &body, None).await?;
        Ok(())
    }

    pub async fn reset_password(&self, code: &str, new_password: &str) -> Result<(), ClientError> {
        let body = ResetConfirmBody { code, new_password };
        self.post("/api/password-reset/confirm", &body, None)
            .await?;
        Ok(())
    }

    pub async fn request_email_change(
        &self,
        session: &str,
        email: &str,
    ) -> Result<(), ClientError> {
        let body = EmailBody { email };
        self.post("/api/me/email", &body, Some(session)).await?;
        Ok(())
    }

    pub async fn confirm_email(&self, code: &str) -> Result<User, ClientError> {
        let body = CodeBody { code };
        let response = self.post("/api/me/email/confirm", &body, None).await?;
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    pub async fn activate(&self, code: &str) -> Result<User, ClientError> {
        let body = CodeBody { code };
        let response = self.post("/api/activate", &body, None).await?;
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    pub async fn deregister(&self, session: &str) -> Result<Option<String>, ClientError> {
        let response = self
            .post("/api/me/deregister", &Nothing {}, Some(session))
            .await?;
        Ok(session_cookie_of(&response))
    }

    pub async fn avatar(&self, username: &str) -> Result<Option<Avatar>, ClientError> {
        let path = format!("/api/users/{}/avatar", encode_path(username));
        let found = self.bytes(&path).await?;
        Ok(found.map(|picture| Avatar {
            content_type: picture.content_type,
            bytes: picture.bytes,
        }))
    }

    async fn bytes(&self, path: &str) -> Result<Option<Picture>, ClientError> {
        let response = self.send(self.http.get(self.address(path)), None).await?;
        let status = response.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_owned())
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        let bytes = response
            .bytes()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))?
            .to_vec();
        Ok(Some(Picture {
            content_type,
            bytes,
        }))
    }

    pub async fn register(&self, body: &RegisterBody) -> Result<Session, ClientError> {
        session_of(self.post("/api/register", body, None).await?).await
    }

    pub async fn sign_in(&self, username: &str, password: &str) -> Result<Session, ClientError> {
        let body = SignInBody {
            username: username.to_owned(),
            password: password.to_owned(),
        };
        session_of(self.post("/api/sign-in", &body, None).await?).await
    }

    pub async fn sign_out(&self, session: &str) -> Result<Option<String>, ClientError> {
        let response = self
            .post("/api/sign-out", &Nothing {}, Some(session))
            .await?;
        Ok(response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|raw| raw.starts_with(&format!("{SESSION_COOKIE}=")))
            .map(|raw| raw.to_owned()))
    }

    async fn feed(&self, path: &str) -> Result<Option<Feed>, ClientError> {
        let response = self.send(self.http.get(self.address(path)), None).await?;
        let status = response.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_owned())
            .unwrap_or_else(|| "application/atom+xml".to_owned());
        let body = response
            .text()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))?;
        Ok(Some(Feed { content_type, body }))
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        session: Option<&str>,
    ) -> Result<T, ClientError> {
        let response = self
            .send(self.http.get(self.address(path)), session)
            .await?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    async fn maybe_get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        session: Option<&str>,
    ) -> Result<Option<T>, ClientError> {
        let response = self
            .send(self.http.get(self.address(path)), session)
            .await?;
        let status = response.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        response
            .json()
            .await
            .map(Some)
            .map_err(|e| ClientError::Detail(e.to_string()))
    }

    async fn post<B: Serialize>(
        &self,
        path: &str,
        body: &B,
        session: Option<&str>,
    ) -> Result<reqwest::Response, ClientError> {
        let response = self
            .send(self.http.post(self.address(path)).json(body), session)
            .await?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        Ok(response)
    }

    async fn patch<B: Serialize>(
        &self,
        path: &str,
        body: &B,
        session: Option<&str>,
    ) -> Result<reqwest::Response, ClientError> {
        let response = self
            .send(self.http.patch(self.address(path)).json(body), session)
            .await?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        Ok(response)
    }

    async fn delete(
        &self,
        path: &str,
        session: Option<&str>,
    ) -> Result<reqwest::Response, ClientError> {
        let response = self
            .send(self.http.delete(self.address(path)), session)
            .await?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(refusal(status, response).await);
        }
        Ok(response)
    }

    fn address(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        session: Option<&str>,
    ) -> Result<reqwest::Response, ClientError> {
        let request = match session {
            Some(token) => request.header("cookie", format!("{SESSION_COOKIE}={token}")),
            None => request,
        };
        request
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))
    }
}

#[derive(Serialize)]
struct BioBody<'a> {
    bio: Option<&'a str>,
}

#[derive(Serialize)]
struct ChangePasswordBody<'a> {
    current_password: &'a str,
    new_password: &'a str,
}

#[derive(Serialize)]
struct EmailBody<'a> {
    email: &'a str,
}

#[derive(Serialize)]
struct DescribeBody<'a> {
    description: &'a str,
}

#[derive(Serialize)]
struct MeansBody<'a> {
    means: &'a str,
}

#[derive(Serialize)]
struct ResetConfirmBody<'a> {
    code: &'a str,
    new_password: &'a str,
}

#[derive(Serialize)]
struct CodeBody<'a> {
    code: &'a str,
}

#[derive(Serialize)]
struct SignInBody {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct Nothing {}

#[derive(Serialize)]
struct RemarkEdit<'a> {
    body: &'a str,
}

#[derive(Serialize)]
struct StickyBody {
    sticky: bool,
}

#[derive(Serialize)]
struct OffFrontBody {
    off_front: bool,
}

#[derive(Serialize)]
struct ResolvedBody {
    resolved: bool,
}

#[derive(Serialize)]
struct ScoreBody {
    postscore: i32,
}

#[derive(Serialize)]
struct MoveBody<'a> {
    group: &'a str,
}

#[derive(Serialize)]
struct ImageBody<'a> {
    data: &'a str,
}

#[derive(Serialize)]
struct PollBody<'a> {
    question: &'a str,
    options: Vec<String>,
}

#[derive(Serialize)]
struct VoteBody<'a> {
    option_id: &'a str,
}

#[derive(Serialize)]
struct ReactBody<'a> {
    kind: &'a str,
}

async fn json_of<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, ClientError> {
    response
        .json()
        .await
        .map_err(|e| ClientError::Detail(e.to_string()))
}

#[derive(Deserialize)]
struct Refusal {
    error: Option<String>,
}

async fn refusal(status: u16, response: reqwest::Response) -> ClientError {
    match response.json::<Refusal>().await {
        Ok(Refusal {
            error: Some(reason),
        }) => ClientError::Rejected { status, reason },
        _ => ClientError::Status(status),
    }
}

fn session_cookie_of(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|raw| raw.starts_with(&format!("{SESSION_COOKIE}=")))
        .map(|raw| raw.to_owned())
}

async fn session_of(response: reqwest::Response) -> Result<Session, ClientError> {
    let cookie = session_cookie_of(&response)
        .ok_or_else(|| ClientError::Detail("the board answered without a session".to_owned()))?;
    let token = cookie
        .split(';')
        .next()
        .unwrap_or("")
        .strip_prefix(&format!("{SESSION_COOKIE}="))
        .unwrap_or("")
        .to_owned();
    Ok(Session { token, cookie })
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
    fn every_narrowing_and_order_is_read_back_by_its_word_and_shown_in_words() {
        for scope in Scope::ALL {
            assert_eq!(Scope::parse(scope.label()), Some(scope));
            assert!(!scope.words().is_empty());
        }
        for order in Order::ALL {
            assert_eq!(Order::parse(order.label()), Some(order));
            assert!(!order.words().is_empty());
        }
        assert_eq!(Scope::parse("everywhere"), None);
        assert_eq!(Order::parse("loudest"), None);
    }

    #[test]
    fn asking_for_nothing_in_particular_searches_everything_by_relevance() {
        let criteria = Criteria::parse("adapters", None, None);
        assert_eq!(
            criteria,
            Criteria::new("adapters", Scope::Everything, Order::Relevance)
        );
        assert_eq!(criteria.query(), "adapters");
    }

    #[test]
    fn a_narrowing_it_does_not_know_falls_back_rather_than_breaking() {
        assert_eq!(
            Criteria::parse("adapters", Some("everywhere"), Some("loudest")),
            Criteria::new("adapters", Scope::Everything, Order::Relevance)
        );
        assert_eq!(
            Criteria::parse("adapters", Some("comments"), Some("oldest")),
            Criteria::new("adapters", Scope::Comments, Order::Oldest)
        );
    }

    #[test]
    fn the_address_of_a_search_carries_all_three_and_keeps_the_words_inside_it() {
        assert_eq!(
            search_address(&Criteria::new("adapters", Scope::Topics, Order::Newest)),
            "/api/search?q=adapters&scope=topics&order=newest"
        );
        assert_eq!(
            search_address(&Criteria::new(
                "a/b?c d",
                Scope::Everything,
                Order::Relevance
            )),
            "/api/search?q=a%2Fb%3Fc%20d&scope=everything&order=relevance"
        );
    }

    #[test]
    fn a_status_is_read_from_every_answer_that_has_one() {
        assert_eq!(ClientError::Status(404).status(), Some(404));
        assert_eq!(
            ClientError::Rejected {
                status: 409,
                reason: "taken".to_owned()
            }
            .status(),
            Some(409)
        );
        assert_eq!(ClientError::Detail("x".to_owned()).status(), None);
        assert_eq!(ClientError::Transport("x".to_owned()).status(), None);
        assert_eq!(ClientError::BadBaseUrl("x".to_owned()).status(), None);
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
        assert_eq!(
            ClientError::Rejected {
                status: 409,
                reason: "username taken".to_owned()
            }
            .to_string(),
            "username taken"
        );
    }
}
