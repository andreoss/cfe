use domain::{
    Comment, CommentId, Email, Query, SearchHit, Section, SectionId, Session, SessionId,
    SessionToken, Slug, Topic, TopicId, User, UserId, Username,
};
use time::OffsetDateTime;

#[async_trait::async_trait]
pub trait UserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User>;
    async fn find_by_email(&self, email: &Email) -> Option<User>;
    async fn find_by_id(&self, id: UserId) -> Option<User>;
    async fn save(&self, user: &User);
    async fn update(&self, user: &User);
    async fn count(&self) -> u64;
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

#[async_trait::async_trait]
pub trait SectionRepository {
    async fn find_by_slug(&self, slug: &Slug) -> Option<Section>;
    async fn find_by_id(&self, id: SectionId) -> Option<Section>;
    async fn list(&self) -> Vec<Section>;
}

#[async_trait::async_trait]
pub trait TopicRepository {
    async fn save(&self, topic: &Topic);
    async fn update(&self, topic: &Topic);
    async fn find_by_id(&self, id: TopicId) -> Option<Topic>;
    async fn list_by_section(&self, section_id: SectionId) -> Vec<Topic>;
    async fn list_by_tag(&self, tag: &Slug) -> Vec<Topic>;
}

#[async_trait::async_trait]
pub trait SearchRepository {
    async fn search(&self, query: &Query) -> Vec<SearchHit>;
}

#[async_trait::async_trait]
pub trait CommentRepository {
    async fn save(&self, comment: &Comment);
    async fn update(&self, comment: &Comment);
    async fn find_by_id(&self, id: CommentId) -> Option<Comment>;
    async fn list_by_topic(&self, topic_id: TopicId) -> Vec<Comment>;
}
