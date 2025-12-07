use crate::ports::{AbuseRepository, Mailer, Message, SessionRepository};
use domain::{Address, User, Username};
use time::{Duration, OffsetDateTime};

pub const SIGN_IN_ATTEMPT_MAX: u64 = 10;
pub const SIGN_IN_ATTEMPT_WINDOW: Duration = Duration::minutes(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignInLimits {
    pub attempt_max: u64,
    pub attempt_window: Duration,
}

impl Default for SignInLimits {
    fn default() -> Self {
        Self {
            attempt_max: SIGN_IN_ATTEMPT_MAX,
            attempt_window: SIGN_IN_ATTEMPT_WINDOW,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TooManyAttempts;

pub async fn end_every_session(sessions: &(impl SessionRepository + ?Sized), user: &User) {
    sessions.delete_for_user(user.id()).await;
}

pub async fn enforce_sign_in_attempts(
    abuse: &(impl AbuseRepository + ?Sized),
    username: &Username,
    addr: &Address,
    now: OffsetDateTime,
    limits: SignInLimits,
) -> Result<(), TooManyAttempts> {
    let since = now - limits.attempt_window;
    let seen = abuse
        .count_sign_in_failures(username.as_str(), addr, since)
        .await;
    if seen >= limits.attempt_max {
        return Err(TooManyAttempts);
    }
    Ok(())
}

pub async fn record_sign_in_failure(
    abuse: &(impl AbuseRepository + ?Sized),
    username: &Username,
    addr: &Address,
    now: OffsetDateTime,
) {
    abuse
        .record_sign_in_failure(username.as_str(), addr, now)
        .await;
}

pub async fn clear_sign_in_failures(abuse: &(impl AbuseRepository + ?Sized), username: &Username) {
    abuse.clear_sign_in_failures(username.as_str()).await;
}

pub async fn notice_of_new_network(
    abuse: &(impl AbuseRepository + ?Sized),
    mailer: &(impl Mailer + ?Sized),
    user: &User,
    addr: &Address,
    now: OffsetDateTime,
) -> bool {
    if abuse.has_seen_address(user.id(), addr).await {
        return false;
    }
    abuse.remember_address(user.id(), addr, now).await;
    mailer
        .send(&Message {
            to: user.email().as_str().to_owned(),
            subject: "A new sign-in".to_owned(),
            body: format!("Your account was used from {}.", addr.as_str()),
        })
        .await;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeAbuseRepo, FakeMailer, FakeSessionRepo};
    use domain::{Email, Session, SessionId, SessionToken, UserId};

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn addr() -> Address {
        Address::parse("203.0.113.10").unwrap()
    }

    fn other_addr() -> Address {
        Address::parse("198.51.100.4").unwrap()
    }

    #[tokio::test]
    async fn ending_every_session_leaves_the_account_signed_out_everywhere() {
        let sessions = FakeSessionRepo::new();
        let owner = user(1, "owner_01");
        for n in 0..3u128 {
            sessions
                .save(&Session::new(
                    SessionId::new(uuid::Uuid::from_u128(100 + n)),
                    owner.id(),
                    SessionToken::parse(&format!("{:064x}", 200 + n)).unwrap(),
                    OffsetDateTime::UNIX_EPOCH + Duration::days(1),
                ))
                .await;
        }
        end_every_session(&sessions, &owner).await;
        let token = SessionToken::parse(&format!("{:064x}", 200)).unwrap();
        assert!(sessions.find_by_token(&token).await.is_none());
    }

    #[tokio::test]
    async fn a_clean_account_may_try_to_sign_in() {
        let abuse = FakeAbuseRepo::new();
        assert_eq!(
            enforce_sign_in_attempts(
                &abuse,
                &Username::parse("owner_01").unwrap(),
                &addr(),
                OffsetDateTime::UNIX_EPOCH,
                SignInLimits::default(),
            )
            .await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn too_many_failures_stop_further_attempts() {
        let abuse = FakeAbuseRepo::new();
        let name = Username::parse("owner_01").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        for _ in 0..SIGN_IN_ATTEMPT_MAX {
            record_sign_in_failure(&abuse, &name, &addr(), now).await;
        }
        assert_eq!(
            enforce_sign_in_attempts(&abuse, &name, &addr(), now, SignInLimits::default()).await,
            Err(TooManyAttempts)
        );
    }

    #[tokio::test]
    async fn failures_outside_the_window_do_not_count() {
        let abuse = FakeAbuseRepo::new();
        let name = Username::parse("owner_01").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        for _ in 0..SIGN_IN_ATTEMPT_MAX {
            record_sign_in_failure(&abuse, &name, &addr(), now).await;
        }
        let later = now + SIGN_IN_ATTEMPT_WINDOW + Duration::seconds(1);
        assert_eq!(
            enforce_sign_in_attempts(&abuse, &name, &addr(), later, SignInLimits::default()).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn a_success_clears_what_was_counted() {
        let abuse = FakeAbuseRepo::new();
        let name = Username::parse("owner_01").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        for _ in 0..SIGN_IN_ATTEMPT_MAX {
            record_sign_in_failure(&abuse, &name, &addr(), now).await;
        }
        clear_sign_in_failures(&abuse, &name).await;
        assert_eq!(
            enforce_sign_in_attempts(&abuse, &name, &addr(), now, SignInLimits::default()).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn another_account_is_not_stopped_by_someone_elses_failures() {
        let abuse = FakeAbuseRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let noisy = Username::parse("noisy_01").unwrap();
        for _ in 0..SIGN_IN_ATTEMPT_MAX {
            record_sign_in_failure(&abuse, &noisy, &addr(), now).await;
        }
        let quiet = Username::parse("quiet_01").unwrap();
        assert_eq!(
            enforce_sign_in_attempts(&abuse, &quiet, &addr(), now, SignInLimits::default()).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn the_first_sign_in_from_an_address_is_noticed() {
        let abuse = FakeAbuseRepo::new();
        let mailer = FakeMailer::new();
        let owner = user(2, "owner_02");
        let noticed =
            notice_of_new_network(&abuse, &mailer, &owner, &addr(), OffsetDateTime::UNIX_EPOCH)
                .await;
        assert!(noticed);
        let sent = mailer.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "owner_02@example.com");
        assert!(sent[0].body.contains("203.0.113.10"));
    }

    #[tokio::test]
    async fn a_familiar_address_is_not_noticed_again() {
        let abuse = FakeAbuseRepo::new();
        let mailer = FakeMailer::new();
        let owner = user(3, "owner_03");
        let now = OffsetDateTime::UNIX_EPOCH;
        notice_of_new_network(&abuse, &mailer, &owner, &addr(), now).await;
        let again = notice_of_new_network(&abuse, &mailer, &owner, &addr(), now).await;
        assert!(!again);
        assert_eq!(mailer.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_different_address_is_noticed_on_its_own() {
        let abuse = FakeAbuseRepo::new();
        let mailer = FakeMailer::new();
        let owner = user(4, "owner_04");
        let now = OffsetDateTime::UNIX_EPOCH;
        notice_of_new_network(&abuse, &mailer, &owner, &addr(), now).await;
        let second = notice_of_new_network(&abuse, &mailer, &owner, &other_addr(), now).await;
        assert!(second);
        assert_eq!(mailer.sent().len(), 2);
    }

    #[tokio::test]
    async fn one_account_seeing_an_address_does_not_speak_for_another() {
        let abuse = FakeAbuseRepo::new();
        let mailer = FakeMailer::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        notice_of_new_network(&abuse, &mailer, &user(5, "owner_05"), &addr(), now).await;
        let other =
            notice_of_new_network(&abuse, &mailer, &user(6, "owner_06"), &addr(), now).await;
        assert!(other);
    }
}
