use crate::ports::{AbuseRepository, Challenge, CommentRepository, TopicRepository};
use domain::{
    Address, AddressBlock, BlockMode, ClientString, Deletion, PostRef, Reason, User, UserId,
};
use time::{Duration, OffsetDateTime};

pub const RATE_LIMIT_MAX: u64 = 120;
pub const ACCOUNT_RATE_LIMIT_MAX: u64 = 5;
pub const RATE_LIMIT_WINDOW: Duration = Duration::minutes(1);
pub const SLOW_MODE_SCORE_FLOOR: i32 = 5;
pub const SLOW_MODE_INTERVAL: Duration = Duration::minutes(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub rate_limit_max: u64,
    pub account_rate_limit_max: u64,
    pub rate_limit_window: Duration,
    pub slow_mode_score_floor: i32,
    pub slow_mode_interval: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            rate_limit_max: RATE_LIMIT_MAX,
            account_rate_limit_max: ACCOUNT_RATE_LIMIT_MAX,
            rate_limit_window: RATE_LIMIT_WINDOW,
            slow_mode_score_floor: SLOW_MODE_SCORE_FLOOR,
            slow_mode_interval: SLOW_MODE_INTERVAL,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AbuseError {
    NotAuthorized,
    AddressBlocked,
    RateLimited,
    SlowMode,
    ChallengeRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChallengeRules {
    pub on_register: bool,
    pub below_floor: bool,
}

impl Default for ChallengeRules {
    fn default() -> Self {
        Self {
            on_register: false,
            below_floor: false,
        }
    }
}

pub async fn enforce_registration_challenge(
    challenge: &(impl Challenge + ?Sized),
    answer: Option<&str>,
    rules: ChallengeRules,
) -> Result<(), AbuseError> {
    if !rules.on_register {
        return Ok(());
    }
    if challenge.verify(answer).await {
        return Ok(());
    }
    Err(AbuseError::ChallengeRequired)
}

pub async fn enforce_posting(
    abuse: &(impl AbuseRepository + ?Sized),
    challenge: &(impl Challenge + ?Sized),
    user: &User,
    addr: &Address,
    answer: Option<&str>,
    now: OffsetDateTime,
    limits: Limits,
    rules: ChallengeRules,
) -> Result<(), AbuseError> {
    let mut challenged = rules.below_floor && user.score().value() < limits.slow_mode_score_floor;
    if let Some(block) = abuse.find_address_block(addr).await {
        if block.is_active_at(now) {
            match block.mode() {
                BlockMode::Refuse => return Err(AbuseError::AddressBlocked),
                BlockMode::Challenge => challenged = true,
                BlockMode::Allow => {}
            }
        }
    }
    let moderator = user.role().is_moderator();
    if challenged && !moderator && !challenge.verify(answer).await {
        return Err(AbuseError::ChallengeRequired);
    }
    if !moderator {
        if user.score().value() < limits.slow_mode_score_floor {
            if let Some(last) = abuse.last_post_by_user(user.id()).await {
                if last > now - limits.slow_mode_interval {
                    return Err(AbuseError::SlowMode);
                }
            }
        }
    }
    if !moderator {
        let since = now - limits.rate_limit_window;
        if abuse.count_posts_by_user(user.id(), since).await >= limits.account_rate_limit_max {
            return Err(AbuseError::RateLimited);
        }
        if abuse.count_posts_by_address(addr, since).await >= limits.rate_limit_max {
            return Err(AbuseError::RateLimited);
        }
    }
    Ok(())
}

pub async fn record_post(
    abuse: &(impl AbuseRepository + ?Sized),
    user_id: UserId,
    addr: &Address,
    client: Option<&ClientString>,
    target: Option<PostRef>,
    at: OffsetDateTime,
) {
    abuse.record_post(user_id, addr, client, target, at).await;
}

pub async fn remove_posts_from_address(
    abuse: &(impl AbuseRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    comments: &(impl CommentRepository + ?Sized),
    moderator: &User,
    addr: &Address,
    since: OffsetDateTime,
    reason: Reason,
    now: OffsetDateTime,
) -> Result<usize, AbuseError> {
    if !moderator.role().is_moderator() {
        return Err(AbuseError::NotAuthorized);
    }
    let deletion = Deletion::new(moderator.id(), reason, domain::Penalty::default(), now);
    let mut removed = 0;
    for reference in abuse.refs_from_address_since(addr, since).await {
        match reference {
            PostRef::Topic(id) => {
                if let Some(topic) = topics.find_by_id(id).await {
                    if topic.is_deleted() {
                        continue;
                    }
                    topics.update(&topic.with_deletion(deletion.clone())).await;
                    removed += 1;
                }
            }
            PostRef::Comment(id) => {
                if let Some(comment) = comments.find_by_id(id).await {
                    if comment.is_deleted() {
                        continue;
                    }
                    comments
                        .update(&comment.with_deletion(deletion.clone()))
                        .await;
                    removed += 1;
                }
            }
        }
    }
    Ok(removed)
}

pub async fn block_address(
    abuse: &(impl AbuseRepository + ?Sized),
    moderator: &User,
    addr: Address,
    reason: Reason,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
    mode: BlockMode,
) -> Result<AddressBlock, AbuseError> {
    if !moderator.role().is_moderator() {
        return Err(AbuseError::NotAuthorized);
    }
    let block =
        AddressBlock::new(addr.clone(), moderator.id(), reason, blocked_at, until).with_mode(mode);
    abuse.save_address_block(&addr, &block).await;
    Ok(block)
}

pub async fn lift_address_block(
    abuse: &(impl AbuseRepository + ?Sized),
    moderator: &User,
    addr: &Address,
) -> Result<(), AbuseError> {
    if !moderator.role().is_moderator() {
        return Err(AbuseError::NotAuthorized);
    }
    abuse.delete_address_block(addr).await;
    Ok(())
}

pub async fn list_address_blocks(abuse: &(impl AbuseRepository + ?Sized)) -> Vec<AddressBlock> {
    abuse.list_address_blocks().await
}

pub async fn is_address_blocked(
    abuse: &(impl AbuseRepository + ?Sized),
    addr: &Address,
    now: OffsetDateTime,
) -> bool {
    match abuse.find_address_block(addr).await {
        Some(block) => block.is_active_at(now),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeAbuseRepo, FakeChallenge};
    use domain::{Email, Score, Username};

    fn plain_user(id: uuid::Uuid) -> User {
        User::register(
            UserId::new(id),
            Username::parse("alice_01").unwrap(),
            Email::parse("alice@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn established_user(id: uuid::Uuid) -> User {
        plain_user(id).with_score(Score::of(SLOW_MODE_SCORE_FLOOR))
    }

    fn addr() -> Address {
        Address::parse("203.0.113.10").unwrap()
    }

    #[tokio::test]
    async fn a_clean_address_and_user_pass() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &plain_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn an_active_address_block_refuses_everyone() {
        let abuse = FakeAbuseRepo::new();
        abuse.insert_block(AddressBlock::new(
            addr(),
            UserId::new(uuid::Uuid::nil()),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        ));
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let now = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &moderator,
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Err(AbuseError::AddressBlocked)
        );
    }

    #[tokio::test]
    async fn a_lapsed_address_block_allows_posting() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.insert_block(AddressBlock::new(
            addr(),
            UserId::new(uuid::Uuid::nil()),
            Reason::parse("spam").unwrap(),
            now,
            Some(now + Duration::hours(1)),
        ));
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &plain_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now + Duration::hours(2),
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_low_standing_author_must_wait_out_slow_mode() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_last_post(UserId::new(uuid::Uuid::nil()), now);
        let user = plain_user(uuid::Uuid::nil());
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &user,
                &addr(),
                None,
                now + Duration::seconds(30),
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Err(AbuseError::SlowMode)
        );
    }

    #[tokio::test]
    async fn slow_mode_clears_after_the_interval() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_last_post(UserId::new(uuid::Uuid::nil()), now);
        let user = plain_user(uuid::Uuid::nil());
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &user,
                &addr(),
                None,
                now + SLOW_MODE_INTERVAL + Duration::seconds(1),
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn an_established_author_is_not_slowed() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_last_post(UserId::new(uuid::Uuid::nil()), now);
        let established = plain_user(uuid::Uuid::nil()).with_score(Score::of(60));
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &established,
                &addr(),
                None,
                now + Duration::seconds(1),
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_moderator_is_not_slowed_or_rated() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_last_post(UserId::new(uuid::Uuid::max()), now);
        abuse.set_address_posts(&addr(), now, RATE_LIMIT_MAX + 5);
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &moderator,
                &addr(),
                None,
                now + Duration::seconds(1),
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn an_address_over_the_rate_is_refused() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_address_posts(&addr(), now, RATE_LIMIT_MAX);
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &established_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Err(AbuseError::RateLimited)
        );
    }

    #[tokio::test]
    async fn a_rate_under_the_limit_passes() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_address_posts(
            &addr(),
            now - RATE_LIMIT_WINDOW - Duration::seconds(1),
            RATE_LIMIT_MAX,
        );
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &established_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_moderator_can_block_and_lift_an_address() {
        let abuse = FakeAbuseRepo::new();
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let now = OffsetDateTime::UNIX_EPOCH;
        let block = block_address(
            &abuse,
            &moderator,
            addr(),
            Reason::parse("flood").unwrap(),
            now,
            None,
            BlockMode::Refuse,
        )
        .await
        .unwrap();
        assert_eq!(block.addr().as_str(), "203.0.113.10");
        assert!(is_address_blocked(&abuse, &addr(), now).await);
        lift_address_block(&abuse, &moderator, &addr())
            .await
            .unwrap();
        assert!(!is_address_blocked(&abuse, &addr(), now).await);
    }

    #[tokio::test]
    async fn a_plain_user_cannot_block_an_address() {
        let abuse = FakeAbuseRepo::new();
        let result = block_address(
            &abuse,
            &plain_user(uuid::Uuid::nil()),
            addr(),
            Reason::parse("flood").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
            None,
            BlockMode::Refuse,
        )
        .await;
        assert_eq!(result, Err(AbuseError::NotAuthorized));
    }

    #[tokio::test]
    async fn a_plain_user_cannot_lift_an_address_block() {
        let abuse = FakeAbuseRepo::new();
        let result = lift_address_block(&abuse, &plain_user(uuid::Uuid::nil()), &addr()).await;
        assert_eq!(result, Err(AbuseError::NotAuthorized));
    }

    #[tokio::test]
    async fn an_account_is_limited_before_the_address_ceiling_bites() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let user = established_user(uuid::Uuid::nil());
        for _ in 0..ACCOUNT_RATE_LIMIT_MAX {
            record_post(&abuse, user.id(), &addr(), None, None, now).await;
        }
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &user,
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Err(AbuseError::RateLimited)
        );
    }

    #[tokio::test]
    async fn one_busy_account_does_not_silence_another_on_the_same_address() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let noisy = established_user(uuid::Uuid::from_u128(1));
        let quiet = established_user(uuid::Uuid::from_u128(2));
        for _ in 0..ACCOUNT_RATE_LIMIT_MAX {
            record_post(&abuse, noisy.id(), &addr(), None, None, now).await;
        }
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &noisy,
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Err(AbuseError::RateLimited)
        );
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &quiet,
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn the_address_ceiling_still_holds_against_many_accounts() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        abuse.set_address_posts(&addr(), now, RATE_LIMIT_MAX);
        let fresh = established_user(uuid::Uuid::from_u128(9));
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &fresh,
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Err(AbuseError::RateLimited)
        );
    }

    #[tokio::test]
    async fn the_address_ceiling_is_looser_than_the_account_limit() {
        assert!(RATE_LIMIT_MAX > ACCOUNT_RATE_LIMIT_MAX);
    }

    #[tokio::test]
    async fn a_recorded_post_keeps_the_client_string_for_investigation() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let user = established_user(uuid::Uuid::nil());
        let client = ClientString::parse("agent/1.0").unwrap();
        record_post(&abuse, user.id(), &addr(), Some(&client), None, now).await;
        let found = abuse
            .posts_from_address(&addr(), domain::Page::first())
            .await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].user_id(), user.id());
        assert_eq!(found[0].client().map(|c| c.as_str()), Some("agent/1.0"));
        assert_eq!(abuse.count_posts_from_address(&addr()).await, 1);
    }

    #[tokio::test]
    async fn a_moderator_removes_a_run_of_posts_from_one_address() {
        use crate::test_support::{FakeCommentRepo, FakeTopicRepo};
        use domain::{Body, Comment, CommentId, SectionId, TagSet, Title, Topic, TopicId};

        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let author = established_user(uuid::Uuid::from_u128(3));
        let topic = Topic::new(
            TopicId::new(uuid::Uuid::from_u128(11)),
            SectionId::new(uuid::Uuid::nil()),
            author.id(),
            Title::parse("Flood").unwrap(),
            Body::parse("A flooded topic").unwrap(),
            TagSet::empty(),
            now,
        );
        let comment = Comment::new(
            CommentId::new(uuid::Uuid::from_u128(12)),
            topic.id(),
            author.id(),
            None,
            Body::parse("A flooded comment").unwrap(),
            now,
        );
        let topics = FakeTopicRepo::with(topic.clone());
        let comments = FakeCommentRepo::new();
        comments.save(&comment).await;

        record_post(
            &abuse,
            author.id(),
            &addr(),
            None,
            Some(PostRef::Topic(topic.id())),
            now,
        )
        .await;
        record_post(
            &abuse,
            author.id(),
            &addr(),
            None,
            Some(PostRef::Comment(comment.id())),
            now,
        )
        .await;

        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let removed = remove_posts_from_address(
            &abuse,
            &topics,
            &comments,
            &moderator,
            &addr(),
            now - Duration::hours(1),
            Reason::parse("a flood").unwrap(),
            now,
        )
        .await
        .unwrap();
        assert_eq!(removed, 2);
        assert!(topics.find_by_id(topic.id()).await.unwrap().is_deleted());
        assert!(
            comments
                .find_by_id(comment.id())
                .await
                .unwrap()
                .is_deleted()
        );
    }

    #[tokio::test]
    async fn removing_again_takes_nothing_more() {
        use crate::test_support::{FakeCommentRepo, FakeTopicRepo};
        use domain::{Body, SectionId, TagSet, Title, Topic, TopicId};

        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let author = established_user(uuid::Uuid::from_u128(4));
        let topic = Topic::new(
            TopicId::new(uuid::Uuid::from_u128(13)),
            SectionId::new(uuid::Uuid::nil()),
            author.id(),
            Title::parse("Once").unwrap(),
            Body::parse("Removed only once").unwrap(),
            TagSet::empty(),
            now,
        );
        let topics = FakeTopicRepo::with(topic.clone());
        let comments = FakeCommentRepo::new();
        record_post(
            &abuse,
            author.id(),
            &addr(),
            None,
            Some(PostRef::Topic(topic.id())),
            now,
        )
        .await;
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let reason = Reason::parse("a flood").unwrap();
        let first = remove_posts_from_address(
            &abuse,
            &topics,
            &comments,
            &moderator,
            &addr(),
            now - Duration::hours(1),
            reason.clone(),
            now,
        )
        .await
        .unwrap();
        let second = remove_posts_from_address(
            &abuse,
            &topics,
            &comments,
            &moderator,
            &addr(),
            now - Duration::hours(1),
            reason,
            now,
        )
        .await
        .unwrap();
        assert_eq!(first, 1);
        assert_eq!(second, 0);
    }

    #[tokio::test]
    async fn removal_leaves_posts_outside_the_window_alone() {
        use crate::test_support::{FakeCommentRepo, FakeTopicRepo};
        use domain::{Body, SectionId, TagSet, Title, Topic, TopicId};

        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let author = established_user(uuid::Uuid::from_u128(5));
        let old = Topic::new(
            TopicId::new(uuid::Uuid::from_u128(14)),
            SectionId::new(uuid::Uuid::nil()),
            author.id(),
            Title::parse("Older").unwrap(),
            Body::parse("Posted long before").unwrap(),
            TagSet::empty(),
            now - Duration::days(2),
        );
        let topics = FakeTopicRepo::with(old.clone());
        let comments = FakeCommentRepo::new();
        record_post(
            &abuse,
            author.id(),
            &addr(),
            None,
            Some(PostRef::Topic(old.id())),
            now - Duration::days(2),
        )
        .await;
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let removed = remove_posts_from_address(
            &abuse,
            &topics,
            &comments,
            &moderator,
            &addr(),
            now - Duration::hours(1),
            Reason::parse("a flood").unwrap(),
            now,
        )
        .await
        .unwrap();
        assert_eq!(removed, 0);
        assert!(!topics.find_by_id(old.id()).await.unwrap().is_deleted());
    }

    #[tokio::test]
    async fn a_plain_user_cannot_remove_a_run_of_posts() {
        use crate::test_support::{FakeCommentRepo, FakeTopicRepo};

        let abuse = FakeAbuseRepo::new();
        let result = remove_posts_from_address(
            &abuse,
            &FakeTopicRepo::new(),
            &FakeCommentRepo::new(),
            &plain_user(uuid::Uuid::nil()),
            &addr(),
            OffsetDateTime::UNIX_EPOCH,
            Reason::parse("a flood").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(AbuseError::NotAuthorized));
    }

    fn challenged_below_floor() -> ChallengeRules {
        ChallengeRules {
            on_register: false,
            below_floor: true,
        }
    }

    #[tokio::test]
    async fn nothing_is_challenged_while_the_rules_are_off() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::expecting("open sesame"),
                &plain_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_low_standing_author_must_answer_when_the_rule_is_on() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let challenge = FakeChallenge::expecting("open sesame");
        assert_eq!(
            enforce_posting(
                &abuse,
                &challenge,
                &plain_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                challenged_below_floor(),
            )
            .await,
            Err(AbuseError::ChallengeRequired)
        );
        assert_eq!(
            enforce_posting(
                &abuse,
                &challenge,
                &plain_user(uuid::Uuid::nil()),
                &addr(),
                Some("open sesame"),
                now,
                Limits::default(),
                challenged_below_floor(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn an_established_author_is_never_challenged_by_standing() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::expecting("open sesame"),
                &established_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                challenged_below_floor(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_block_in_challenge_mode_asks_instead_of_refusing() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        abuse.insert_block(
            AddressBlock::new(
                addr(),
                UserId::new(uuid::Uuid::nil()),
                Reason::parse("a shared address").unwrap(),
                now,
                None,
            )
            .with_mode(BlockMode::Challenge),
        );
        let challenge = FakeChallenge::expecting("open sesame");
        let user = established_user(uuid::Uuid::nil());
        assert_eq!(
            enforce_posting(
                &abuse,
                &challenge,
                &user,
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default(),
            )
            .await,
            Err(AbuseError::ChallengeRequired)
        );
        assert_eq!(
            enforce_posting(
                &abuse,
                &challenge,
                &user,
                &addr(),
                Some("open sesame"),
                now,
                Limits::default(),
                ChallengeRules::default(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_block_in_allow_mode_lets_the_write_through() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        abuse.insert_block(
            AddressBlock::new(
                addr(),
                UserId::new(uuid::Uuid::nil()),
                Reason::parse("watched only").unwrap(),
                now,
                None,
            )
            .with_mode(BlockMode::Allow),
        );
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::expecting("open sesame"),
                &established_user(uuid::Uuid::nil()),
                &addr(),
                None,
                now,
                Limits::default(),
                ChallengeRules::default(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_moderator_is_never_challenged() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        abuse.insert_block(
            AddressBlock::new(
                addr(),
                UserId::new(uuid::Uuid::nil()),
                Reason::parse("a shared address").unwrap(),
                now,
                None,
            )
            .with_mode(BlockMode::Challenge),
        );
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::expecting("open sesame"),
                &moderator,
                &addr(),
                None,
                now,
                Limits::default(),
                challenged_below_floor(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn registration_asks_only_when_the_rule_is_on() {
        let challenge = FakeChallenge::expecting("open sesame");
        assert_eq!(
            enforce_registration_challenge(&challenge, None, ChallengeRules::default()).await,
            Ok(())
        );
        let rules = ChallengeRules {
            on_register: true,
            below_floor: false,
        };
        assert_eq!(
            enforce_registration_challenge(&challenge, None, rules).await,
            Err(AbuseError::ChallengeRequired)
        );
        assert_eq!(
            enforce_registration_challenge(&challenge, Some("open sesame"), rules).await,
            Ok(())
        );
        assert_eq!(
            enforce_registration_challenge(&challenge, Some("wrong"), rules).await,
            Err(AbuseError::ChallengeRequired)
        );
    }

    #[tokio::test]
    async fn recording_a_post_is_visible_to_the_limits() {
        let abuse = FakeAbuseRepo::new();
        let user = established_user(uuid::Uuid::nil());
        let now = OffsetDateTime::UNIX_EPOCH;
        record_post(&abuse, user.id(), &addr(), None, None, now).await;
        assert_eq!(
            enforce_posting(
                &abuse,
                &FakeChallenge::accepting(),
                &user,
                &addr(),
                None,
                now + Duration::seconds(1),
                Limits::default(),
                ChallengeRules::default()
            )
            .await,
            Ok(())
        );
        assert_eq!(abuse.count_posts_by_address(&addr(), now).await, 1);
    }
}
