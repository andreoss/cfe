use crate::{Bio, Email, Username};

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
    bio: Option<Bio>,
}

impl User {
    pub fn register(id: UserId, username: Username, email: Email, password_hash: String) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            bio: None,
        }
    }

    pub fn from_parts(
        id: UserId,
        username: Username,
        email: Email,
        password_hash: String,
        bio: Option<Bio>,
    ) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            bio,
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

    pub fn bio(&self) -> Option<&Bio> {
        self.bio.as_ref()
    }

    pub fn with_bio(&self, bio: Option<Bio>) -> Self {
        Self {
            bio,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_given_fields_and_no_bio() {
        let id = UserId::new(uuid::Uuid::nil());
        let username = Username::parse("alice_01").unwrap();
        let email = Email::parse("alice@example.com").unwrap();
        let user = User::register(id, username.clone(), email.clone(), "hash".to_owned());
        assert_eq!(user.id(), id);
        assert_eq!(user.username(), &username);
        assert_eq!(user.email(), &email);
        assert_eq!(user.password_hash(), "hash");
        assert_eq!(user.bio(), None);
    }

    #[test]
    fn with_bio_replaces_bio_and_keeps_identity() {
        let id = UserId::new(uuid::Uuid::nil());
        let username = Username::parse("alice_01").unwrap();
        let email = Email::parse("alice@example.com").unwrap();
        let user = User::register(id, username, email, "hash".to_owned());
        let bio = Bio::parse("hello").unwrap();
        let updated = user.with_bio(bio.clone());
        assert_eq!(updated.id(), user.id());
        assert_eq!(updated.bio(), bio.as_ref());
    }
}
