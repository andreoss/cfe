use domain::{
    Avatar, Bookmark, Comment, CommentId, Email, Notification, NotificationId, Poll, PollId,
    PollOptionId,
    Query, Reaction,
    ReactionKind, ReactionTarget, SearchHit, Section, SectionId, Session, SessionId, SessionToken,
    Slug, Topic, TopicId, User, UserId, Username, Vote,
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
    async fn delete_for_user(&self, user_id: UserId);
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
pub trait AvatarRepository {
    async fn save(&self, user_id: UserId, avatar: &Avatar);
    async fn find_by_user(&self, user_id: UserId) -> Option<Avatar>;
    async fn delete(&self, user_id: UserId);
}

#[async_trait::async_trait]
pub trait BookmarkRepository {
    async fn save(&self, bookmark: &Bookmark);
    async fn delete(&self, user_id: UserId, topic_id: TopicId);
    async fn exists(&self, user_id: UserId, topic_id: TopicId) -> bool;
    async fn list_topics(&self, user_id: UserId) -> Vec<Topic>;
}

#[async_trait::async_trait]
pub trait PollRepository {
    async fn save(&self, poll: &Poll);
    async fn find_by_topic(&self, topic_id: TopicId) -> Option<Poll>;
    async fn save_vote(&self, vote: &Vote);
    async fn counts(&self, poll_id: PollId) -> Vec<(PollOptionId, u64)>;
    async fn find_vote(&self, poll_id: PollId, user_id: UserId) -> Option<PollOptionId>;
}

#[async_trait::async_trait]
pub trait ReactionRepository {
    async fn save(&self, reaction: &Reaction);
    async fn delete(&self, user_id: UserId, target: ReactionTarget);
    async fn counts(&self, target: ReactionTarget) -> Vec<(ReactionKind, u64)>;
    async fn find_mine(&self, user_id: UserId, target: ReactionTarget) -> Option<ReactionKind>;
}

#[async_trait::async_trait]
pub trait NotificationRepository {
    async fn save(&self, notification: &Notification);
    async fn update(&self, notification: &Notification);
    async fn find_by_id(&self, id: NotificationId) -> Option<Notification>;
    async fn list_by_recipient(&self, recipient_id: UserId) -> Vec<Notification>;
    async fn count_unread(&self, recipient_id: UserId) -> u64;
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
