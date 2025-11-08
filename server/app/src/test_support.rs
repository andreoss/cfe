#![cfg(test)]

use crate::ports::{PasswordHasher, SessionRepository, UserRepository};
use domain::{Email, Session, SessionId, SessionToken, User, UserId, Username};
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
