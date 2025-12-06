use domain::{
    Address, AddressBlock, AddressPost, Attachment, AttachmentId, Avatar, Ban, Bookmark,
    ClientString, Comment, CommentId, ContentItem, Email, Group, GroupId, Invitation,
    InvitationCode, InvitationId, MailToken, Notification, NotificationId, Page, Poll, PollId,
    PollOptionId, PostRef, Query, Reaction, ReactionKind, ReactionTarget, Remark, Report, ReportId,
    ReportTarget, Section, SectionId, Session, SessionId, SessionToken, Slug, Tag, Topic, TopicId,
    User, UserId, Username, Version, VersionId, VersionOf, Vote, Warning, Watch,
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
    async fn find_at_or_below_score(&self, score: i32) -> Vec<User>;
    async fn find_unconfirmed_before(&self, cutoff: OffsetDateTime) -> Vec<User>;
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
    async fn save(&self, section: &Section);
    async fn update(&self, section: &Section);
    async fn find_by_slug(&self, slug: &Slug) -> Option<Section>;
    async fn find_by_id(&self, id: SectionId) -> Option<Section>;
    async fn list(&self) -> Vec<Section>;
}

#[async_trait::async_trait]
pub trait TagRepository {
    async fn save(&self, tag: &Tag);
    async fn find(&self, slug: &Slug) -> Option<Tag>;
    async fn list(&self) -> Vec<Tag>;
    async fn follow(&self, user_id: UserId, slug: &Slug);
    async fn unfollow(&self, user_id: UserId, slug: &Slug);
    async fn is_following(&self, user_id: UserId, slug: &Slug) -> bool;
    async fn followed_by(&self, user_id: UserId) -> Vec<Slug>;
    async fn followers(&self, slug: &Slug) -> Vec<UserId>;
}

#[async_trait::async_trait]
pub trait TopicRepository {
    async fn all_for_archive(&self) -> Vec<Topic>;
    async fn list_between(
        &self,
        from: OffsetDateTime,
        until: OffsetDateTime,
        page: Page,
    ) -> Vec<Topic>;
    async fn count_between(&self, from: OffsetDateTime, until: OffsetDateTime) -> u64;
    async fn save(&self, topic: &Topic);
    async fn update(&self, topic: &Topic);
    async fn find_by_id(&self, id: TopicId) -> Option<Topic>;
    async fn list_by_section(&self, section_id: SectionId, page: Page) -> Vec<Topic>;
    async fn count_by_section(&self, section_id: SectionId) -> u64;
    async fn list_by_tag(&self, tag: &Slug, page: Page) -> Vec<Topic>;
    async fn count_by_tag(&self, tag: &Slug) -> u64;
}

#[async_trait::async_trait]
pub trait GroupRepository {
    async fn save(&self, group: &Group);
    async fn update(&self, group: &Group);
    async fn find_by_id(&self, id: GroupId) -> Option<Group>;
    async fn find_by_slug(&self, section_id: SectionId, slug: &Slug) -> Option<Group>;
    async fn list_by_section(&self, section_id: SectionId) -> Vec<Group>;
}

#[async_trait::async_trait]
pub trait ActivityRepository {
    async fn recent(&self, limit: u32) -> Vec<ContentItem>;
}

#[async_trait::async_trait]
pub trait EnforcementRepository {
    async fn save_ban(&self, user_id: UserId, ban: &Ban);
    async fn find_ban(&self, user_id: UserId) -> Option<Ban>;
    async fn delete_ban(&self, user_id: UserId);
    async fn save_warning(&self, warning: &Warning);
    async fn list_warnings(&self, user_id: UserId) -> Vec<Warning>;
    async fn save_ignore(&self, user_id: UserId, ignored_id: UserId);
    async fn delete_ignore(&self, user_id: UserId, ignored_id: UserId);
    async fn list_ignored(&self, user_id: UserId) -> Vec<UserId>;
}

#[async_trait::async_trait]
pub trait AbuseRepository {
    async fn find_address_block(&self, addr: &Address) -> Option<AddressBlock>;
    async fn save_address_block(&self, addr: &Address, block: &AddressBlock);
    async fn delete_address_block(&self, addr: &Address);
    async fn list_address_blocks(&self) -> Vec<AddressBlock>;
    async fn count_posts_by_address(&self, addr: &Address, since: OffsetDateTime) -> u64;
    async fn count_posts_by_user(&self, user_id: UserId, since: OffsetDateTime) -> u64;
    async fn last_post_by_user(&self, user_id: UserId) -> Option<OffsetDateTime>;
    async fn record_post(
        &self,
        user_id: UserId,
        addr: &Address,
        client: Option<&ClientString>,
        target: Option<PostRef>,
        at: OffsetDateTime,
    );
    async fn refs_from_address_since(&self, addr: &Address, since: OffsetDateTime) -> Vec<PostRef>;
    async fn record_sign_in_failure(&self, username: &str, addr: &Address, at: OffsetDateTime);
    async fn count_sign_in_failures(
        &self,
        username: &str,
        addr: &Address,
        since: OffsetDateTime,
    ) -> u64;
    async fn clear_sign_in_failures(&self, username: &str);
    async fn has_seen_address(&self, user_id: UserId, addr: &Address) -> bool;
    async fn remember_address(&self, user_id: UserId, addr: &Address, at: OffsetDateTime);
    async fn posts_from_address(&self, addr: &Address, page: Page) -> Vec<AddressPost>;
    async fn count_posts_from_address(&self, addr: &Address) -> u64;
}

#[async_trait::async_trait]
pub trait AttachmentRepository {
    async fn save(&self, attachment: &Attachment);
    async fn find(&self, id: AttachmentId) -> Option<Attachment>;
    async fn list_for(&self, topic_id: TopicId) -> Vec<Attachment>;
    async fn count_for(&self, topic_id: TopicId) -> u64;
    async fn delete(&self, id: AttachmentId);
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
    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic>;
    async fn count_topics(&self, user_id: UserId) -> u64;
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
    async fn list_by_recipient(&self, recipient_id: UserId, page: Page) -> Vec<Notification>;
    async fn count_by_recipient(&self, recipient_id: UserId) -> u64;
    async fn count_unread(&self, recipient_id: UserId) -> u64;
}

#[async_trait::async_trait]
pub trait SearchRepository {
    async fn search(&self, query: &Query) -> Vec<ContentItem>;
}

#[async_trait::async_trait]
pub trait VersionRepository {
    async fn save(&self, version: &Version);
    async fn list_for(&self, of: VersionOf, subject_id: uuid::Uuid) -> Vec<Version>;
    async fn find(&self, id: VersionId) -> Option<Version>;
}

#[async_trait::async_trait]
pub trait CommentRepository {
    async fn save(&self, comment: &Comment);
    async fn update(&self, comment: &Comment);
    async fn find_by_id(&self, id: CommentId) -> Option<Comment>;
    async fn list_by_topic(&self, topic_id: TopicId, page: Page) -> Vec<Comment>;
    async fn count_roots(&self, topic_id: TopicId) -> u64;
}

pub struct Message {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait::async_trait]
pub trait Mailer {
    async fn send(&self, message: &Message);
}

pub trait TokenDigest {
    fn digest(&self, secret: &str) -> String;
}

#[async_trait::async_trait]
pub trait MailTokenRepository {
    async fn save(&self, token: &MailToken);
    async fn find_by_digest(&self, digest: &str) -> Option<MailToken>;
}

#[async_trait::async_trait]
pub trait ReportRepository {
    async fn save(&self, report: &Report);
    async fn update(&self, report: &Report);
    async fn find_by_id(&self, id: ReportId) -> Option<Report>;
    async fn list_open(&self, page: Page) -> Vec<Report>;
    async fn count_open(&self) -> u64;
    async fn count_open_for_topic(&self, topic_id: TopicId) -> u64;
    async fn find_open_by_reporter(
        &self,
        reporter_id: UserId,
        target: ReportTarget,
    ) -> Option<Report>;
    async fn count_by_reporter_since(&self, reporter_id: UserId, since: OffsetDateTime) -> u64;
}

#[async_trait::async_trait]
pub trait Challenge {
    async fn verify(&self, answer: Option<&str>) -> bool;
}

#[async_trait::async_trait]
pub trait WatchRepository {
    async fn save(&self, watch: &Watch);
    async fn delete(&self, user_id: UserId, topic_id: TopicId);
    async fn find(&self, user_id: UserId, topic_id: TopicId) -> Option<Watch>;
    async fn watchers(&self, topic_id: TopicId) -> Vec<UserId>;
    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic>;
    async fn count_topics(&self, user_id: UserId) -> u64;
}

#[async_trait::async_trait]
pub trait InvitationRepository {
    async fn save(&self, invitation: &Invitation);
    async fn claim(&self, id: InvitationId, at: OffsetDateTime) -> bool;
    async fn attribute(&self, id: InvitationId, by: UserId);
    async fn release(&self, id: InvitationId);
    async fn find_by_code(&self, code: &InvitationCode) -> Option<Invitation>;
    async fn list_by_issuer(&self, issuer_id: UserId, page: Page) -> Vec<Invitation>;
    async fn count_by_issuer(&self, issuer_id: UserId) -> u64;
    async fn count_outstanding(&self, issuer_id: UserId, now: OffsetDateTime) -> u64;
}

#[async_trait::async_trait]
pub trait RemarkRepository {
    async fn save(&self, remark: &Remark);
    async fn delete(&self, author_id: UserId, subject_id: UserId);
    async fn find(&self, author_id: UserId, subject_id: UserId) -> Option<Remark>;
    async fn list_by_author(&self, author_id: UserId, page: Page) -> Vec<Remark>;
    async fn count_by_author(&self, author_id: UserId) -> u64;
}
