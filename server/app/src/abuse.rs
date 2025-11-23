use crate::ports::AbuseRepository;
use domain::{Address, AddressBlock, ClientString, Reason, User, UserId};
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
}

pub async fn enforce_posting(
    abuse: &(impl AbuseRepository + ?Sized),
    user: &User,
    addr: &Address,
    now: OffsetDateTime,
    limits: Limits,
) -> Result<(), AbuseError> {
    if let Some(block) = abuse.find_address_block(addr).await {
        if block.is_active_at(now) {
            return Err(AbuseError::AddressBlocked);
        }
    }
    let moderator = user.role().is_moderator();
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
    at: OffsetDateTime,
) {
    abuse.record_post(user_id, addr, client, at).await;
}

pub async fn block_address(
    abuse: &(impl AbuseRepository + ?Sized),
    moderator: &User,
    addr: Address,
    reason: Reason,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
) -> Result<AddressBlock, AbuseError> {
    if !moderator.role().is_moderator() {
        return Err(AbuseError::NotAuthorized);
    }
    let block = AddressBlock::new(addr.clone(), moderator.id(), reason, blocked_at, until);
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

pub async fn list_address_blocks(
    abuse: &(impl AbuseRepository + ?Sized),
) -> Vec<AddressBlock> {
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
    use crate::test_support::FakeAbuseRepo;
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
            enforce_posting(&abuse, &plain_user(uuid::Uuid::nil()), &addr(), now, Limits::default()).await,
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
            enforce_posting(&abuse, &moderator, &addr(), now, Limits::default()).await,
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
            enforce_posting(&abuse, &plain_user(uuid::Uuid::nil()), &addr(), now + Duration::hours(2), Limits::default()).await,
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
            enforce_posting(&abuse, &user, &addr(), now + Duration::seconds(30), Limits::default()).await,
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
            enforce_posting(&abuse, &user, &addr(), now + SLOW_MODE_INTERVAL + Duration::seconds(1), Limits::default()).await,
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
            enforce_posting(&abuse, &established, &addr(), now + Duration::seconds(1), Limits::default()).await,
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
            enforce_posting(&abuse, &moderator, &addr(), now + Duration::seconds(1), Limits::default()).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn an_address_over_the_rate_is_refused() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_address_posts(&addr(), now, RATE_LIMIT_MAX);
        assert_eq!(
            enforce_posting(&abuse, &established_user(uuid::Uuid::nil()), &addr(), now, Limits::default()).await,
            Err(AbuseError::RateLimited)
        );
    }

    #[tokio::test]
    async fn a_rate_under_the_limit_passes() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let abuse = FakeAbuseRepo::new();
        abuse.set_address_posts(&addr(), now - RATE_LIMIT_WINDOW - Duration::seconds(1), RATE_LIMIT_MAX);
        assert_eq!(
            enforce_posting(&abuse, &established_user(uuid::Uuid::nil()), &addr(), now, Limits::default()).await,
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
        )
        .await
        .unwrap();
        assert_eq!(block.addr().as_str(), "203.0.113.10");
        assert!(is_address_blocked(&abuse, &addr(), now).await);
        lift_address_block(&abuse, &moderator, &addr()).await.unwrap();
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
            record_post(&abuse, user.id(), &addr(), None, now).await;
        }
        assert_eq!(
            enforce_posting(&abuse, &user, &addr(), now, Limits::default()).await,
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
            record_post(&abuse, noisy.id(), &addr(), None, now).await;
        }
        assert_eq!(
            enforce_posting(&abuse, &noisy, &addr(), now, Limits::default()).await,
            Err(AbuseError::RateLimited)
        );
        assert_eq!(
            enforce_posting(&abuse, &quiet, &addr(), now, Limits::default()).await,
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
            enforce_posting(&abuse, &fresh, &addr(), now, Limits::default()).await,
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
        record_post(&abuse, user.id(), &addr(), Some(&client), now).await;
        let found = abuse.posts_from_address(&addr(), domain::Page::first()).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].user_id(), user.id());
        assert_eq!(found[0].client().map(|c| c.as_str()), Some("agent/1.0"));
        assert_eq!(abuse.count_posts_from_address(&addr()).await, 1);
    }

    #[tokio::test]
    async fn recording_a_post_is_visible_to_the_limits() {
        let abuse = FakeAbuseRepo::new();
        let user = established_user(uuid::Uuid::nil());
        let now = OffsetDateTime::UNIX_EPOCH;
        record_post(&abuse, user.id(), &addr(), None, now).await;
        assert_eq!(
            enforce_posting(&abuse, &user, &addr(), now + Duration::seconds(1), Limits::default()).await,
            Ok(())
        );
        assert_eq!(
            abuse.count_posts_by_address(&addr(), now).await,
            1
        );
    }
}