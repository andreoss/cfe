use crate::{Email, Username};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    username: Username,
    email: Email,
    password_hash: String,
}

impl User {
    pub fn register(id: UserId, username: Username, email: Email, password_hash: String) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
        }
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn username(&self) -> &Username {
        &self.username
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_given_fields() {
        let id = UserId::new(uuid::Uuid::nil());
        let username = Username::parse("alice_01").unwrap();
        let email = Email::parse("alice@example.com").unwrap();
        let user = User::register(id, username.clone(), email.clone(), "hash".to_owned());
        assert_eq!(user.id(), id);
        assert_eq!(user.username(), &username);
        assert_eq!(user.email(), &email);
        assert_eq!(user.password_hash(), "hash");
    }
}
