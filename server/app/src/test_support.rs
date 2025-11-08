#![cfg(test)]

use crate::ports::{PasswordHasher, UserRepository};
use domain::{Email, User, Username};
use std::sync::Mutex;

pub struct FakeRepo {
    users: Mutex<Vec<User>>,
}

impl FakeRepo {
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
impl UserRepository for FakeRepo {
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
