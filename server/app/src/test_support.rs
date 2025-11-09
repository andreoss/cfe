#![cfg(test)]

use crate::ports::{
    PasswordHasher, SectionRepository, SessionRepository, TopicRepository, UserRepository,
};
use domain::{
    Email, Section, SectionId, Session, SessionId, SessionToken, Slug, Topic, TopicId, User,
    UserId, Username,
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
}
