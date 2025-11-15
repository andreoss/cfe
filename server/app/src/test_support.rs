#![cfg(test)]

use crate::ports::{
    BookmarkRepository, CommentRepository, NotificationRepository, PasswordHasher,
    SearchRepository, SectionRepository, SessionRepository, TopicRepository, UserRepository,
};
use domain::{
    Body, Bookmark, Comment, CommentId, Email, Notification, NotificationId, Query, SearchHit,
    Section, SectionId, Session, SessionId, SessionToken, Slug, TagSet, Title, Topic, TopicId,
    User, UserId, Username,
};
use std::sync::Mutex;
use time::OffsetDateTime;

pub struct FakeUserRepo {
    users: Mutex<Vec<User>>,
}

impl FakeUserRepo {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(Vec::new()),
        }
    }

    pub fn with(user: User) -> Self {
        Self {
            users: Mutex::new(vec![user]),
        }
    }
}

#[async_trait::async_trait]
impl UserRepository for FakeUserRepo {
    async fn find_by_username(&self, username: &Username) -> Option<User> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.username() == username)
            .cloned()
    }

    async fn find_by_email(&self, email: &Email) -> Option<User> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.email() == email)
            .cloned()
    }

    async fn find_by_id(&self, id: UserId) -> Option<User> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id() == id)
            .cloned()
    }

    async fn save(&self, user: &User) {
        self.users.lock().unwrap().push(user.clone());
    }

    async fn update(&self, user: &User) {
        let mut users = self.users.lock().unwrap();
        if let Some(existing) = users.iter_mut().find(|u| u.id() == user.id()) {
            *existing = user.clone();
        }
    }

    async fn count(&self) -> u64 {
        self.users.lock().unwrap().len() as u64
    }
}

pub struct FakeHasher;

impl PasswordHasher for FakeHasher {
    fn hash(&self, plain: &str) -> String {
        format!("hashed:{plain}")
    }

    fn verify(&self, plain: &str, hash: &str) -> bool {
        hash == format!("hashed:{plain}")
    }
}

pub struct FakeSessionRepo {
    sessions: Mutex<Vec<Session>>,
}

impl FakeSessionRepo {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(Vec::new()),
        }
    }

    pub fn with(session: Session) -> Self {
        Self {
            sessions: Mutex::new(vec![session]),
        }
    }
}

#[async_trait::async_trait]
impl SessionRepository for FakeSessionRepo {
    async fn save(&self, session: &Session) {
        self.sessions.lock().unwrap().push(session.clone());
    }

    async fn find_by_token(&self, token: &SessionToken) -> Option<Session> {
        self.sessions
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.token() == token)
            .cloned()
    }

    async fn touch(&self, id: SessionId, new_expiry: OffsetDateTime) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.iter_mut().find(|s| s.id() == id) {
            *session = session.extended(new_expiry);
        }
    }

    async fn delete(&self, id: SessionId) {
        self.sessions.lock().unwrap().retain(|s| s.id() != id);
    }
}

pub struct FakeSectionRepo {
    sections: Mutex<Vec<Section>>,
}

impl FakeSectionRepo {
    pub fn new() -> Self {
        Self {
            sections: Mutex::new(Vec::new()),
        }
    }

    pub fn with(section: Section) -> Self {
        Self {
            sections: Mutex::new(vec![section]),
        }
    }
}

#[async_trait::async_trait]
impl SectionRepository for FakeSectionRepo {
    async fn find_by_slug(&self, slug: &Slug) -> Option<Section> {
        self.sections
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.slug() == slug)
            .cloned()
    }

    async fn find_by_id(&self, id: SectionId) -> Option<Section> {
        self.sections
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id() == id)
            .cloned()
    }

    async fn list(&self) -> Vec<Section> {
        self.sections.lock().unwrap().clone()
    }
}

pub struct FakeTopicRepo {
    topics: Mutex<Vec<Topic>>,
}

impl FakeTopicRepo {
    pub fn new() -> Self {
        Self {
            topics: Mutex::new(Vec::new()),
        }
    }

    pub fn with(topic: Topic) -> Self {
        Self {
            topics: Mutex::new(vec![topic]),
        }
    }
}

#[async_trait::async_trait]
impl TopicRepository for FakeTopicRepo {
    async fn save(&self, topic: &Topic) {
        self.topics.lock().unwrap().push(topic.clone());
    }

    async fn find_by_id(&self, id: TopicId) -> Option<Topic> {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id() == id)
            .cloned()
    }

    async fn list_by_section(&self, section_id: SectionId) -> Vec<Topic> {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.section_id() == section_id)
            .cloned()
            .collect()
    }

    async fn list_by_tag(&self, tag: &Slug) -> Vec<Topic> {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.tags().contains(tag))
            .cloned()
            .collect()
    }

    async fn update(&self, topic: &Topic) {
        let mut topics = self.topics.lock().unwrap();
        if let Some(existing) = topics.iter_mut().find(|t| t.id() == topic.id()) {
            *existing = topic.clone();
        }
    }
}

pub struct FakeCommentRepo {
    comments: Mutex<Vec<Comment>>,
}

impl FakeCommentRepo {
    pub fn new() -> Self {
        Self {
            comments: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl CommentRepository for FakeCommentRepo {
    async fn save(&self, comment: &Comment) {
        self.comments.lock().unwrap().push(comment.clone());
    }

    async fn find_by_id(&self, id: CommentId) -> Option<Comment> {
        self.comments
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.id() == id)
            .cloned()
    }

    async fn list_by_topic(&self, topic_id: TopicId) -> Vec<Comment> {
        self.comments
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.topic_id() == topic_id)
            .cloned()
            .collect()
    }

    async fn update(&self, comment: &Comment) {
        let mut comments = self.comments.lock().unwrap();
        if let Some(existing) = comments.iter_mut().find(|c| c.id() == comment.id()) {
            *existing = comment.clone();
        }
    }
}

pub struct FakeSearchRepo {
    hits: Vec<SearchHit>,
    last_query: Mutex<Option<String>>,
}

impl FakeSearchRepo {
    pub fn with(hits: Vec<SearchHit>) -> Self {
        Self {
            hits,
            last_query: Mutex::new(None),
        }
    }

    pub fn last_query(&self) -> Option<String> {
        self.last_query.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl SearchRepository for FakeSearchRepo {
    async fn search(&self, query: &Query) -> Vec<SearchHit> {
        *self.last_query.lock().unwrap() = Some(query.as_str().to_owned());
        self.hits.clone()
    }
}

pub struct FakeNotificationRepo {
    notifications: Mutex<Vec<Notification>>,
}

impl FakeNotificationRepo {
    pub fn new() -> Self {
        Self {
            notifications: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl NotificationRepository for FakeNotificationRepo {
    async fn save(&self, notification: &Notification) {
        self.notifications.lock().unwrap().push(notification.clone());
    }

    async fn update(&self, notification: &Notification) {
        let mut all = self.notifications.lock().unwrap();
        if let Some(slot) = all.iter_mut().find(|n| n.id() == notification.id()) {
            *slot = notification.clone();
        }
    }

    async fn find_by_id(&self, id: NotificationId) -> Option<Notification> {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .find(|n| n.id() == id)
            .cloned()
    }

    async fn list_by_recipient(&self, recipient_id: UserId) -> Vec<Notification> {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.recipient_id() == recipient_id)
            .cloned()
            .collect()
    }

    async fn count_unread(&self, recipient_id: UserId) -> u64 {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.recipient_id() == recipient_id && !n.is_read())
            .count() as u64
    }
}

pub struct FakeBookmarkRepo {
    bookmarks: Mutex<Vec<Bookmark>>,
    topics: Mutex<Vec<Topic>>,
}

impl FakeBookmarkRepo {
    pub fn new() -> Self {
        Self {
            bookmarks: Mutex::new(Vec::new()),
            topics: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl BookmarkRepository for FakeBookmarkRepo {
    async fn save(&self, bookmark: &Bookmark) {
        let mut all = self.bookmarks.lock().unwrap();
        if all
            .iter()
            .any(|b| b.user_id() == bookmark.user_id() && b.topic_id() == bookmark.topic_id())
        {
            return;
        }
        all.push(bookmark.clone());
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        self.bookmarks
            .lock()
            .unwrap()
            .retain(|b| !(b.user_id() == user_id && b.topic_id() == topic_id));
    }

    async fn exists(&self, user_id: UserId, topic_id: TopicId) -> bool {
        self.bookmarks
            .lock()
            .unwrap()
            .iter()
            .any(|b| b.user_id() == user_id && b.topic_id() == topic_id)
    }

    async fn list_topics(&self, user_id: UserId) -> Vec<Topic> {
        let known = self.topics.lock().unwrap();
        self.bookmarks
            .lock()
            .unwrap()
            .iter()
            .filter(|b| b.user_id() == user_id)
            .map(|b| {
                known
                    .iter()
                    .find(|t| t.id() == b.topic_id())
                    .cloned()
                    .unwrap_or_else(|| placeholder_topic(b.topic_id()))
            })
            .collect()
    }
}

fn placeholder_topic(id: TopicId) -> Topic {
    Topic::new(
        id,
        SectionId::new(uuid::Uuid::nil()),
        UserId::new(uuid::Uuid::max()),
        Title::parse("Saved").unwrap(),
        Body::parse("Saved").unwrap(),
        TagSet::empty(),
        OffsetDateTime::UNIX_EPOCH,
    )
}
