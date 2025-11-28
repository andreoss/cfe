use crate::{Bio, Email, Role, Score, Username};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }

    pub fn scheduled_work() -> Self {
        Self(uuid::Uuid::nil())
    }

    pub fn is_scheduled_work(&self) -> bool {
        self.0.is_nil()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    username: Username,
    email: Email,
    password_hash: String,
    bio: Option<Bio>,
    role: Role,
    deregistered_at: Option<OffsetDateTime>,
    confirmed_at: Option<OffsetDateTime>,
    score: Score,
    registered_at: OffsetDateTime,
}

impl User {
    pub fn register(id: UserId, username: Username, email: Email, password_hash: String) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            bio: None,
            role: Role::User,
            deregistered_at: None,
            confirmed_at: None,
            score: Score::initial(),
            registered_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    pub fn from_parts(
        id: UserId,
        username: Username,
        email: Email,
        password_hash: String,
        bio: Option<Bio>,
        role: Role,
        deregistered_at: Option<OffsetDateTime>,
        confirmed_at: Option<OffsetDateTime>,
        score: Score,
        registered_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            bio,
            role,
            deregistered_at,
            confirmed_at,
            score,
            registered_at,
        }
    }

    pub fn registered_at(&self) -> OffsetDateTime {
        self.registered_at
    }

    pub fn registered(&self, at: OffsetDateTime) -> Self {
        Self {
            registered_at: at,
            ..self.clone()
        }
    }

    pub fn score(&self) -> Score {
        self.score
    }

    pub fn with_score(&self, score: Score) -> Self {
        Self {
            score,
            ..self.clone()
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

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn promoted_to_moderator(&self) -> Self {
        Self {
            role: Role::Moderator,
            ..self.clone()
        }
    }

    pub fn with_role(&self, role: Role) -> Self {
        Self {
            role,
            ..self.clone()
        }
    }

    pub fn with_email(&self, email: Email) -> Self {
        Self {
            email,
            ..self.clone()
        }
    }

    pub fn confirmed_at(&self) -> Option<OffsetDateTime> {
        self.confirmed_at
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirmed_at.is_some()
    }

    pub fn confirmed(&self, at: OffsetDateTime) -> Self {
        Self {
            confirmed_at: Some(at),
            ..self.clone()
        }
    }

    pub fn with_password_hash(&self, password_hash: String) -> Self {
        Self {
            password_hash,
            ..self.clone()
        }
    }

    pub fn deregistered_at(&self) -> Option<OffsetDateTime> {
        self.deregistered_at
    }

    pub fn is_active(&self) -> bool {
        self.deregistered_at.is_none()
    }

    pub fn deregistered(&self, at: OffsetDateTime) -> Self {
        Self {
            bio: None,
            deregistered_at: Some(at),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_given_fields_no_bio_and_user_role() {
        let id = UserId::new(uuid::Uuid::nil());
        let username = Username::parse("alice_01").unwrap();
        let email = Email::parse("alice@example.com").unwrap();
        let user = User::register(id, username.clone(), email.clone(), "hash".to_owned());
        assert_eq!(user.id(), id);
        assert_eq!(user.username(), &username);
        assert_eq!(user.email(), &email);
        assert_eq!(user.password_hash(), "hash");
        assert_eq!(user.bio(), None);
        assert_eq!(user.role(), Role::User);
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

    #[test]
    fn a_new_user_is_active() {
        let user = User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("alice_01").unwrap(),
            Email::parse("a@example.com").unwrap(),
            "hash".to_owned(),
        );
        assert!(user.is_active());
        assert_eq!(user.deregistered_at(), None);
    }

    #[test]
    fn deregistering_clears_the_bio_and_keeps_identity() {
        let user = User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("alice_01").unwrap(),
            Email::parse("a@example.com").unwrap(),
            "hash".to_owned(),
        )
        .with_bio(Bio::parse("hello").unwrap());
        let gone = user.deregistered(OffsetDateTime::UNIX_EPOCH);
        assert_eq!(gone.id(), user.id());
        assert_eq!(gone.username(), user.username());
        assert!(!gone.is_active());
        assert_eq!(gone.deregistered_at(), Some(OffsetDateTime::UNIX_EPOCH));
        assert_eq!(gone.bio(), None);
    }

    #[test]
    fn with_password_hash_replaces_only_the_hash() {
        let user = User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("alice_01").unwrap(),
            Email::parse("a@example.com").unwrap(),
            "old".to_owned(),
        );
        let changed = user.with_password_hash("new".to_owned());
        assert_eq!(changed.password_hash(), "new");
        assert_eq!(changed.id(), user.id());
        assert_eq!(changed.username(), user.username());
    }

    #[test]
    fn promoted_to_moderator_changes_role_and_keeps_identity() {
        let id = UserId::new(uuid::Uuid::nil());
        let username = Username::parse("alice_01").unwrap();
        let email = Email::parse("alice@example.com").unwrap();
        let user = User::register(id, username, email, "hash".to_owned());
        let promoted = user.promoted_to_moderator();
        assert_eq!(promoted.id(), user.id());
        assert_eq!(promoted.role(), Role::Moderator);
        assert_eq!(user.role(), Role::User);
    }
}
