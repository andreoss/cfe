use domain::{Email, Session, SessionId, SessionToken, User, UserId, Username};
use time::OffsetDateTime;

#[async_trait::async_trait]
pub trait UserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User>;
    async fn find_by_email(&self, email: &Email) -> Option<User>;
    async fn find_by_id(&self, id: UserId) -> Option<User>;
    async fn save(&self, user: &User);
}

pub trait PasswordHasher {
    fn hash(&self, plain: &str) -> String;
    fn verify(&self, plain: &str, hash: &str) -> bool;
}

#[async_trait::async_trait]
pub trait SessionRepository {
    async fn save(&self, session: &Session);
    async fn find_by_token(&self, token: &SessionToken) -> Option<Session>;
    async fn touch(&self, id: SessionId, new_expiry: OffsetDateTime);
    async fn delete(&self, id: SessionId);
}
