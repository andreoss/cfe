#![cfg(test)]

use crate::ports::{
    AbuseRepository, ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository,
    MailTokenRepository, Mailer, Message, NotificationRepository, PasswordHasher, PollRepository,
    TokenDigest,
    ReactionRepository, SearchRepository, SectionRepository, SessionRepository, TopicRepository,
    UserRepository, GroupRepository,
};
use domain::{
    Address, AddressBlock, Avatar, Ban, Body, Bookmark, Comment, CommentId, Email, MailToken,
    Notification, NotificationId, Page,
    Query,
    Reaction, Warning,
    ReactionKind, ReactionTarget, ContentItem, Section, SectionId, Session, SessionId, SessionToken,
    Poll, PollId, PollOptionId, Slug, TagSet, Title, Topic, TopicId, User, UserId, Username, Vote,
    Group, GroupId,
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

    async fn count(&self) -> u64 {
        self.users.lock().unwrap().len() as u64
    }

    async fn find_at_or_below_score(&self, score: i32) -> Vec<User> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.score().value() <= score)
            .cloned()
            .collect()
    }

    async fn find_unconfirmed_before(&self, cutoff: OffsetDateTime) -> Vec<User> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| !u.is_confirmed() && u.registered_at() < cutoff)
            .cloned()
            .collect()
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

    async fn delete_for_user(&self, user_id: UserId) {
        self.sessions
            .lock()
            .unwrap()
            .retain(|s| s.user_id() != user_id);
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

    async fn find_by_id(&self, id: SectionId) -> Option<Section> {
        self.sections
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id() == id)
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

    pub fn with(topic: Topic) -> Self {
        Self {
            topics: Mutex::new(vec![topic]),
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

    async fn list_by_section(&self, section_id: SectionId, page: Page) -> Vec<Topic> {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.section_id() == section_id)
            .skip(page.offset() as usize)
            .take(page.limit() as usize)
            .cloned()
            .collect()
    }

    async fn count_by_section(&self, section_id: SectionId) -> u64 {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.section_id() == section_id)
            .count() as u64
    }

    async fn list_by_tag(&self, tag: &Slug, page: Page) -> Vec<Topic> {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.tags().contains(tag))
            .skip(page.offset() as usize)
            .take(page.limit() as usize)
            .cloned()
            .collect()
    }

    async fn count_by_tag(&self, tag: &Slug) -> u64 {
        self.topics
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.tags().contains(tag))
            .count() as u64
    }

    async fn update(&self, topic: &Topic) {
        let mut topics = self.topics.lock().unwrap();
        if let Some(existing) = topics.iter_mut().find(|t| t.id() == topic.id()) {
            *existing = topic.clone();
        }
    }
}

pub struct FakeGroupRepo {
    groups: Mutex<Vec<Group>>,
}

impl FakeGroupRepo {
    pub fn new() -> Self {
        Self {
            groups: Mutex::new(Vec::new()),
        }
    }

}

#[async_trait::async_trait]
impl GroupRepository for FakeGroupRepo {
    async fn save(&self, group: &Group) {
        self.groups.lock().unwrap().push(group.clone());
    }

    async fn find_by_id(&self, id: GroupId) -> Option<Group> {
        self.groups
            .lock()
            .unwrap()
            .iter()
            .find(|g| g.id() == id)
            .cloned()
    }

    async fn find_by_slug(&self, section_id: SectionId, slug: &Slug) -> Option<Group> {
        self.groups
            .lock()
            .unwrap()
            .iter()
            .find(|g| g.section_id() == section_id && g.slug() == slug)
            .cloned()
    }

    async fn list_by_section(&self, section_id: SectionId) -> Vec<Group> {
        self.groups
            .lock()
            .unwrap()
            .iter()
            .filter(|g| g.section_id() == section_id)
            .cloned()
            .collect()
    }
}

pub struct FakeCommentRepo {
    comments: Mutex<Vec<Comment>>,
}

impl FakeCommentRepo {
    pub fn new() -> Self {
        Self {
            comments: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl CommentRepository for FakeCommentRepo {
    async fn save(&self, comment: &Comment) {
        self.comments.lock().unwrap().push(comment.clone());
    }

    async fn find_by_id(&self, id: CommentId) -> Option<Comment> {
        self.comments
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.id() == id)
            .cloned()
    }

    async fn list_by_topic(&self, topic_id: TopicId, page: Page) -> Vec<Comment> {
        let all = self.comments.lock().unwrap();
        let roots: Vec<CommentId> = all
            .iter()
            .filter(|c| c.topic_id() == topic_id && c.parent_id().is_none())
            .skip(page.offset() as usize)
            .take(page.limit() as usize)
            .map(|c| c.id())
            .collect();
        all.iter()
            .filter(|c| {
                c.topic_id() == topic_id
                    && match c.parent_id() {
                        None => roots.contains(&c.id()),
                        Some(parent) => roots.contains(&parent),
                    }
            })
            .cloned()
            .collect()
    }

    async fn count_roots(&self, topic_id: TopicId) -> u64 {
        self.comments
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.topic_id() == topic_id && c.parent_id().is_none())
            .count() as u64
    }

    async fn update(&self, comment: &Comment) {
        let mut comments = self.comments.lock().unwrap();
        if let Some(existing) = comments.iter_mut().find(|c| c.id() == comment.id()) {
            *existing = comment.clone();
        }
    }
}

pub struct FakeSearchRepo {
    hits: Vec<ContentItem>,
    last_query: Mutex<Option<String>>,
}

impl FakeSearchRepo {
    pub fn with(hits: Vec<ContentItem>) -> Self {
        Self {
            hits,
            last_query: Mutex::new(None),
        }
    }

    pub fn last_query(&self) -> Option<String> {
        self.last_query.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl SearchRepository for FakeSearchRepo {
    async fn search(&self, query: &Query) -> Vec<ContentItem> {
        *self.last_query.lock().unwrap() = Some(query.as_str().to_owned());
        self.hits.clone()
    }
}

pub struct FakeNotificationRepo {
    notifications: Mutex<Vec<Notification>>,
}

impl FakeNotificationRepo {
    pub fn new() -> Self {
        Self {
            notifications: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl NotificationRepository for FakeNotificationRepo {
    async fn save(&self, notification: &Notification) {
        self.notifications.lock().unwrap().push(notification.clone());
    }

    async fn update(&self, notification: &Notification) {
        let mut all = self.notifications.lock().unwrap();
        if let Some(slot) = all.iter_mut().find(|n| n.id() == notification.id()) {
            *slot = notification.clone();
        }
    }

    async fn find_by_id(&self, id: NotificationId) -> Option<Notification> {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .find(|n| n.id() == id)
            .cloned()
    }

    async fn list_by_recipient(&self, recipient_id: UserId, page: Page) -> Vec<Notification> {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.recipient_id() == recipient_id)
            .skip(page.offset() as usize)
            .take(page.limit() as usize)
            .cloned()
            .collect()
    }

    async fn count_by_recipient(&self, recipient_id: UserId) -> u64 {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.recipient_id() == recipient_id)
            .count() as u64
    }

    async fn count_unread(&self, recipient_id: UserId) -> u64 {
        self.notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.recipient_id() == recipient_id && !n.is_read())
            .count() as u64
    }
}

pub struct FakeBookmarkRepo {
    bookmarks: Mutex<Vec<Bookmark>>,
    topics: Mutex<Vec<Topic>>,
}

impl FakeBookmarkRepo {
    pub fn new() -> Self {
        Self {
            bookmarks: Mutex::new(Vec::new()),
            topics: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl BookmarkRepository for FakeBookmarkRepo {
    async fn save(&self, bookmark: &Bookmark) {
        let mut all = self.bookmarks.lock().unwrap();
        if all
            .iter()
            .any(|b| b.user_id() == bookmark.user_id() && b.topic_id() == bookmark.topic_id())
        {
            return;
        }
        all.push(bookmark.clone());
    }

    async fn delete(&self, user_id: UserId, topic_id: TopicId) {
        self.bookmarks
            .lock()
            .unwrap()
            .retain(|b| !(b.user_id() == user_id && b.topic_id() == topic_id));
    }

    async fn exists(&self, user_id: UserId, topic_id: TopicId) -> bool {
        self.bookmarks
            .lock()
            .unwrap()
            .iter()
            .any(|b| b.user_id() == user_id && b.topic_id() == topic_id)
    }

    async fn count_topics(&self, user_id: UserId) -> u64 {
        self.bookmarks
            .lock()
            .unwrap()
            .iter()
            .filter(|b| b.user_id() == user_id)
            .count() as u64
    }

    async fn list_topics(&self, user_id: UserId, page: Page) -> Vec<Topic> {
        let known = self.topics.lock().unwrap();
        self.bookmarks
            .lock()
            .unwrap()
            .iter()
            .filter(|b| b.user_id() == user_id)
            .skip(page.offset() as usize)
            .take(page.limit() as usize)
            .map(|b| {
                known
                    .iter()
                    .find(|t| t.id() == b.topic_id())
                    .cloned()
                    .unwrap_or_else(|| placeholder_topic(b.topic_id()))
            })
            .collect()
    }
}

fn placeholder_topic(id: TopicId) -> Topic {
    Topic::new(
        id,
        SectionId::new(uuid::Uuid::nil()),
        UserId::new(uuid::Uuid::max()),
        Title::parse("Saved").unwrap(),
        Body::parse("Saved").unwrap(),
        TagSet::empty(),
        OffsetDateTime::UNIX_EPOCH,
    )
}

pub struct FakeReactionRepo {
    reactions: Mutex<Vec<Reaction>>,
}

impl FakeReactionRepo {
    pub fn new() -> Self {
        Self {
            reactions: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl ReactionRepository for FakeReactionRepo {
    async fn save(&self, reaction: &Reaction) {
        let mut all = self.reactions.lock().unwrap();
        all.retain(|r| !(r.user_id() == reaction.user_id() && r.target() == reaction.target()));
        all.push(reaction.clone());
    }

    async fn delete(&self, user_id: UserId, target: ReactionTarget) {
        self.reactions
            .lock()
            .unwrap()
            .retain(|r| !(r.user_id() == user_id && r.target() == target));
    }

    async fn counts(&self, target: ReactionTarget) -> Vec<(ReactionKind, u64)> {
        let all = self.reactions.lock().unwrap();
        ReactionKind::all()
            .into_iter()
            .filter_map(|kind| {
                let count = all
                    .iter()
                    .filter(|r| r.target() == target && r.kind() == kind)
                    .count() as u64;
                (count > 0).then_some((kind, count))
            })
            .collect()
    }

    async fn find_mine(&self, user_id: UserId, target: ReactionTarget) -> Option<ReactionKind> {
        self.reactions
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.user_id() == user_id && r.target() == target)
            .map(|r| r.kind())
    }
}

pub struct FakePollRepo {
    polls: Mutex<Vec<Poll>>,
    votes: Mutex<Vec<Vote>>,
}

impl FakePollRepo {
    pub fn new() -> Self {
        Self {
            polls: Mutex::new(Vec::new()),
            votes: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl PollRepository for FakePollRepo {
    async fn save(&self, poll: &Poll) {
        self.polls.lock().unwrap().push(poll.clone());
    }

    async fn find_by_topic(&self, topic_id: TopicId) -> Option<Poll> {
        self.polls
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.topic_id() == topic_id)
            .cloned()
    }

    async fn save_vote(&self, vote: &Vote) {
        let mut votes = self.votes.lock().unwrap();
        votes.retain(|v| !(v.poll_id() == vote.poll_id() && v.user_id() == vote.user_id()));
        votes.push(vote.clone());
    }

    async fn counts(&self, poll_id: PollId) -> Vec<(PollOptionId, u64)> {
        let votes = self.votes.lock().unwrap();
        let polls = self.polls.lock().unwrap();
        let poll = polls.iter().find(|p| p.id() == poll_id);
        match poll {
            Some(poll) => poll
                .options()
                .iter()
                .map(|o| {
                    let count = votes
                        .iter()
                        .filter(|v| v.poll_id() == poll_id && v.option_id() == o.id())
                        .count() as u64;
                    (o.id(), count)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    async fn find_vote(&self, poll_id: PollId, user_id: UserId) -> Option<PollOptionId> {
        self.votes
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.poll_id() == poll_id && v.user_id() == user_id)
            .map(|v| v.option_id())
    }
}

pub struct FakeAvatarRepo {
    avatars: Mutex<Vec<(UserId, Avatar)>>,
}

impl FakeAvatarRepo {
    pub fn new() -> Self {
        Self {
            avatars: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl AvatarRepository for FakeAvatarRepo {
    async fn save(&self, user_id: UserId, avatar: &Avatar) {
        let mut all = self.avatars.lock().unwrap();
        all.retain(|(id, _)| *id != user_id);
        all.push((user_id, avatar.clone()));
    }

    async fn find_by_user(&self, user_id: UserId) -> Option<Avatar> {
        self.avatars
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| *id == user_id)
            .map(|(_, a)| a.clone())
    }

    async fn delete(&self, user_id: UserId) {
        self.avatars.lock().unwrap().retain(|(id, _)| *id != user_id);
    }
}

pub struct FakeEnforcementRepo {
    bans: Mutex<Vec<(UserId, Ban)>>,
    warnings: Mutex<Vec<Warning>>,
    ignores: Mutex<Vec<(UserId, UserId)>>,
}

impl FakeEnforcementRepo {
    pub fn new() -> Self {
        Self {
            bans: Mutex::new(Vec::new()),
            warnings: Mutex::new(Vec::new()),
            ignores: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl EnforcementRepository for FakeEnforcementRepo {
    async fn save_ban(&self, user_id: UserId, ban: &Ban) {
        let mut bans = self.bans.lock().unwrap();
        bans.retain(|(id, _)| *id != user_id);
        bans.push((user_id, ban.clone()));
    }

    async fn find_ban(&self, user_id: UserId) -> Option<Ban> {
        self.bans
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| *id == user_id)
            .map(|(_, b)| b.clone())
    }

    async fn delete_ban(&self, user_id: UserId) {
        self.bans.lock().unwrap().retain(|(id, _)| *id != user_id);
    }

    async fn save_warning(&self, warning: &Warning) {
        let mut warnings = self.warnings.lock().unwrap();
        warnings.retain(|w| w.id() != warning.id());
        warnings.push(warning.clone());
    }

    async fn list_warnings(&self, user_id: UserId) -> Vec<Warning> {
        self.warnings
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.user_id() == user_id)
            .cloned()
            .collect()
    }

    async fn save_ignore(&self, user_id: UserId, ignored_id: UserId) {
        let mut ignores = self.ignores.lock().unwrap();
        if !ignores.iter().any(|(a, b)| *a == user_id && *b == ignored_id) {
            ignores.push((user_id, ignored_id));
        }
    }

    async fn delete_ignore(&self, user_id: UserId, ignored_id: UserId) {
        self.ignores
            .lock()
            .unwrap()
            .retain(|(a, b)| !(*a == user_id && *b == ignored_id));
    }

    async fn list_ignored(&self, user_id: UserId) -> Vec<UserId> {
        self.ignores
            .lock()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == user_id)
            .map(|(_, b)| *b)
            .collect()
    }
}

pub struct FakeActivityRepo {
    items: Vec<ContentItem>,
    last_limit: Mutex<Option<u32>>,
}

impl FakeActivityRepo {
    pub fn with(items: Vec<ContentItem>) -> Self {
        Self {
            items,
            last_limit: Mutex::new(None),
        }
    }

    pub fn last_limit(&self) -> Option<u32> {
        *self.last_limit.lock().unwrap()
    }
}

#[async_trait::async_trait]
impl ActivityRepository for FakeActivityRepo {
    async fn recent(&self, limit: u32) -> Vec<ContentItem> {
        *self.last_limit.lock().unwrap() = Some(limit);
        self.items.clone()
    }
}

pub struct PlainDigest;

impl TokenDigest for PlainDigest {
    fn digest(&self, secret: &str) -> String {
        format!("digest:{secret}")
    }
}

pub struct FakeMailer {
    sent: Mutex<Vec<(String, String, String)>>,
}

pub struct SentMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl FakeMailer {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }

    pub fn sent(&self) -> Vec<SentMessage> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .map(|(to, subject, body)| SentMessage {
                to: to.clone(),
                subject: subject.clone(),
                body: body.clone(),
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl Mailer for FakeMailer {
    async fn send(&self, message: &Message) {
        self.sent.lock().unwrap().push((
            message.to.clone(),
            message.subject.clone(),
            message.body.clone(),
        ));
    }
}

pub struct FakeMailTokenRepo {
    tokens: Mutex<Vec<MailToken>>,
}

impl FakeMailTokenRepo {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(Vec::new()),
        }
    }

    pub fn all(&self) -> Vec<MailToken> {
        self.tokens.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl MailTokenRepository for FakeMailTokenRepo {
    async fn save(&self, token: &MailToken) {
        let mut all = self.tokens.lock().unwrap();
        all.retain(|t| t.id() != token.id());
        all.push(token.clone());
    }

    async fn find_by_digest(&self, digest: &str) -> Option<MailToken> {
        self.tokens
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.digest() == digest)
            .cloned()
    }
}

pub struct FakeAbuseRepo {
    blocks: Mutex<Vec<AddressBlock>>,
    posts: Mutex<Vec<(UserId, Address, OffsetDateTime)>>,
}

impl FakeAbuseRepo {
    pub fn new() -> Self {
        Self {
            blocks: Mutex::new(Vec::new()),
            posts: Mutex::new(Vec::new()),
        }
    }

    pub fn insert_block(&self, block: AddressBlock) {
        let mut blocks = self.blocks.lock().unwrap();
        blocks.retain(|b| b.addr() != block.addr());
        blocks.push(block);
    }

    pub fn set_last_post(&self, user_id: UserId, at: OffsetDateTime) {
        self.posts.lock().unwrap().push((user_id, Address::parse("0.0.0.0").unwrap(), at));
    }

    pub fn set_address_posts(&self, addr: &Address, at: OffsetDateTime, count: u64) {
        let mut posts = self.posts.lock().unwrap();
        for _ in 0..count {
            posts.push((UserId::new(uuid::Uuid::nil()), addr.clone(), at));
        }
    }
}

#[async_trait::async_trait]
impl AbuseRepository for FakeAbuseRepo {
    async fn find_address_block(&self, addr: &Address) -> Option<AddressBlock> {
        self.blocks
            .lock()
            .unwrap()
            .iter()
            .find(|b| b.addr() == addr)
            .cloned()
    }

    async fn save_address_block(&self, addr: &Address, block: &AddressBlock) {
        self.insert_block(block.clone());
        let _ = addr;
    }

    async fn delete_address_block(&self, addr: &Address) {
        let mut blocks = self.blocks.lock().unwrap();
        blocks.retain(|b| b.addr() != addr);
    }

    async fn list_address_blocks(&self) -> Vec<AddressBlock> {
        self.blocks.lock().unwrap().clone()
    }

    async fn count_posts_by_address(&self, addr: &Address, since: OffsetDateTime) -> u64 {
        self.posts
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, a, at)| a == addr && *at >= since)
            .count() as u64
    }

    async fn last_post_by_user(&self, user_id: UserId) -> Option<OffsetDateTime> {
        self.posts
            .lock()
            .unwrap()
            .iter()
            .filter(|(u, _, _)| *u == user_id)
            .map(|(_, _, at)| *at)
            .max()
    }

    async fn record_post(&self, user_id: UserId, addr: &Address, at: OffsetDateTime) {
        self.posts.lock().unwrap().push((user_id, addr.clone(), at));
    }
}
