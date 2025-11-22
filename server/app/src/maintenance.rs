use crate::ports::{EnforcementRepository, UserRepository};
use domain::{Ban, Reason, SCORE_MIN, UserId};
use time::{Duration, OffsetDateTime};

pub const FALLEN_REASON: &str = "standing fell to the floor";

pub async fn settle_standing(
    users: &(impl UserRepository + ?Sized),
    enforcement: &(impl EnforcementRepository + ?Sized),
    floor: i32,
    now: OffsetDateTime,
) -> Vec<UserId> {
    let reason = Reason::parse(FALLEN_REASON).expect("a valid reason");
    let mut blocked = Vec::new();
    for user in users.find_at_or_below_score(floor).await {
        if !user.is_active() || user.role().is_moderator() {
            continue;
        }
        if enforcement.find_ban(user.id()).await.is_some() {
            continue;
        }
        enforcement
            .save_ban(
                user.id(),
                &Ban::new(UserId::scheduled_work(), reason.clone(), now, None),
            )
            .await;
        blocked.push(user.id());
    }
    blocked
}

pub fn default_floor() -> i32 {
    SCORE_MIN
}

pub const CONFIRMATION_WINDOW: Duration = Duration::days(7);

pub async fn drop_unconfirmed(
    users: &(impl UserRepository + ?Sized),
    window: Duration,
    now: OffsetDateTime,
) -> Vec<UserId> {
    let cutoff = now - window;
    let mut dropped = Vec::new();
    for user in users.find_unconfirmed_before(cutoff).await {
        if !user.is_active() || user.is_confirmed() {
            continue;
        }
        users.update(&user.deregistered(now)).await;
        dropped.push(user.id());
    }
    dropped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeEnforcementRepo, FakeUserRepo};
    use domain::{Email, Score, User, Username};

    fn user(id: u128, name: &str, score: i32) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
        .with_score(Score::of(score))
    }

    async fn repos(users: Vec<User>) -> (FakeUserRepo, FakeEnforcementRepo) {
        let repo = FakeUserRepo::new();
        for u in users {
            repo.save(&u).await;
        }
        (repo, FakeEnforcementRepo::new())
    }

    #[tokio::test]
    async fn blocks_an_account_that_reached_the_floor() {
        let (users, bans) = repos(vec![user(1, "fallen_01", SCORE_MIN)]).await;
        let blocked = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        assert_eq!(blocked.len(), 1);
        assert!(bans.find_ban(blocked[0]).await.is_some());
    }

    #[tokio::test]
    async fn leaves_an_account_above_the_floor_alone() {
        let (users, bans) = repos(vec![user(2, "steady_01", 0)]).await;
        let blocked = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        assert!(blocked.is_empty());
    }

    #[tokio::test]
    async fn never_blocks_a_moderator() {
        let moderator = user(3, "keeper_01", SCORE_MIN).promoted_to_moderator();
        let (users, bans) = repos(vec![moderator]).await;
        let blocked = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        assert!(blocked.is_empty());
    }

    #[tokio::test]
    async fn skips_an_account_that_is_already_banned() {
        let fallen = user(4, "again_01", SCORE_MIN);
        let (users, bans) = repos(vec![fallen.clone()]).await;
        bans.save_ban(
            fallen.id(),
            &Ban::new(
                UserId::scheduled_work(),
                Reason::parse("earlier").unwrap(),
                OffsetDateTime::UNIX_EPOCH,
                None,
            ),
        )
        .await;
        let blocked = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        assert!(blocked.is_empty());
    }

    #[tokio::test]
    async fn skips_an_account_that_left() {
        let gone = user(5, "gone_01", SCORE_MIN).deregistered(OffsetDateTime::UNIX_EPOCH);
        let (users, bans) = repos(vec![gone]).await;
        let blocked = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        assert!(blocked.is_empty());
    }

    #[tokio::test]
    async fn running_twice_blocks_once() {
        let (users, bans) = repos(vec![user(6, "twice_01", SCORE_MIN)]).await;
        let first = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        let second = settle_standing(&users, &bans, SCORE_MIN, OffsetDateTime::UNIX_EPOCH).await;
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn the_default_floor_is_the_lowest_a_score_can_reach() {
        assert_eq!(default_floor(), SCORE_MIN);
    }

    fn day(n: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::days(n)
    }

    #[tokio::test]
    async fn drops_a_registration_never_confirmed_past_the_window() {
        let stale = user(10, "stale_01", 0).registered(day(0));
        let (users, _) = repos(vec![stale.clone()]).await;
        let dropped = drop_unconfirmed(&users, Duration::days(7), day(8)).await;
        assert_eq!(dropped, vec![stale.id()]);
        let after = users.find_by_id(stale.id()).await.unwrap();
        assert!(!after.is_active());
    }

    #[tokio::test]
    async fn keeps_a_registration_still_inside_the_window() {
        let fresh = user(11, "fresh_01", 0).registered(day(5));
        let (users, _) = repos(vec![fresh.clone()]).await;
        let dropped = drop_unconfirmed(&users, Duration::days(7), day(8)).await;
        assert!(dropped.is_empty());
        assert!(users.find_by_id(fresh.id()).await.unwrap().is_active());
    }

    #[tokio::test]
    async fn keeps_a_confirmed_account_however_old() {
        let confirmed = user(12, "conf_01", 0).registered(day(0)).confirmed(day(1));
        let (users, _) = repos(vec![confirmed.clone()]).await;
        let dropped = drop_unconfirmed(&users, Duration::days(7), day(90)).await;
        assert!(dropped.is_empty());
        assert!(users.find_by_id(confirmed.id()).await.unwrap().is_active());
    }

    #[tokio::test]
    async fn dropping_twice_reports_once() {
        let stale = user(13, "twice_02", 0).registered(day(0));
        let (users, _) = repos(vec![stale]).await;
        let first = drop_unconfirmed(&users, Duration::days(7), day(8)).await;
        let second = drop_unconfirmed(&users, Duration::days(7), day(9)).await;
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[tokio::test]
    async fn the_confirmation_window_is_a_week() {
        assert_eq!(CONFIRMATION_WINDOW, Duration::days(7));
    }
}
