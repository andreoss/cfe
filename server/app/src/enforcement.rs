use crate::ports::{EnforcementRepository, SessionRepository, UserRepository};
use domain::{Ban, Reason, User, UserId, Warning, WarningId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum EnforcementError {
    NotAuthorized,
    UserNotFound,
    NotYourself,
}

fn require_moderator(actor: &User) -> Result<(), EnforcementError> {
    if actor.role().is_moderator() {
        Ok(())
    } else {
        Err(EnforcementError::NotAuthorized)
    }
}

pub async fn ban_user(
    users: &(impl UserRepository + ?Sized),
    sessions: &(impl SessionRepository + ?Sized),
    enforcement: &(impl EnforcementRepository + ?Sized),
    moderator: &User,
    target_id: UserId,
    reason: Reason,
    now: OffsetDateTime,
    until: Option<OffsetDateTime>,
) -> Result<Ban, EnforcementError> {
    require_moderator(moderator)?;
    if moderator.id() == target_id {
        return Err(EnforcementError::NotYourself);
    }
    users
        .find_by_id(target_id)
        .await
        .ok_or(EnforcementError::UserNotFound)?;
    let ban = Ban::new(moderator.id(), reason, now, until);
    enforcement.save_ban(target_id, &ban).await;
    sessions.delete_for_user(target_id).await;
    Ok(ban)
}

pub async fn lift_ban(
    enforcement: &(impl EnforcementRepository + ?Sized),
    moderator: &User,
    target_id: UserId,
) -> Result<(), EnforcementError> {
    require_moderator(moderator)?;
    enforcement.delete_ban(target_id).await;
    Ok(())
}

pub async fn active_ban(
    enforcement: &(impl EnforcementRepository + ?Sized),
    user_id: UserId,
    now: OffsetDateTime,
) -> Option<Ban> {
    enforcement
        .find_ban(user_id)
        .await
        .filter(|b| b.is_active_at(now))
}

pub async fn promote_to_moderator(
    users: &(impl UserRepository + ?Sized),
    moderator: &User,
    target_id: UserId,
) -> Result<User, EnforcementError> {
    require_moderator(moderator)?;
    let target = users
        .find_by_id(target_id)
        .await
        .ok_or(EnforcementError::UserNotFound)?;
    let promoted = target.promoted_to_moderator();
    users.update(&promoted).await;
    Ok(promoted)
}

pub async fn warn_user(
    users: &(impl UserRepository + ?Sized),
    enforcement: &(impl EnforcementRepository + ?Sized),
    moderator: &User,
    id: WarningId,
    target_id: UserId,
    reason: Reason,
    now: OffsetDateTime,
) -> Result<Warning, EnforcementError> {
    require_moderator(moderator)?;
    if moderator.id() == target_id {
        return Err(EnforcementError::NotYourself);
    }
    users
        .find_by_id(target_id)
        .await
        .ok_or(EnforcementError::UserNotFound)?;
    let warning = Warning::new(id, target_id, moderator.id(), reason, now);
    enforcement.save_warning(&warning).await;
    Ok(warning)
}

pub async fn list_warnings(
    enforcement: &(impl EnforcementRepository + ?Sized),
    user_id: UserId,
) -> Vec<Warning> {
    enforcement.list_warnings(user_id).await
}

pub async fn acknowledge_warnings(enforcement: &(impl EnforcementRepository + ?Sized), user_id: UserId) {
    for warning in enforcement.list_warnings(user_id).await {
        if !warning.is_acknowledged() {
            enforcement.save_warning(&warning.acknowledged()).await;
        }
    }
}

pub async fn ignore_user(
    users: &(impl UserRepository + ?Sized),
    enforcement: &(impl EnforcementRepository + ?Sized),
    actor: &User,
    target_id: UserId,
) -> Result<(), EnforcementError> {
    if actor.id() == target_id {
        return Err(EnforcementError::NotYourself);
    }
    users
        .find_by_id(target_id)
        .await
        .ok_or(EnforcementError::UserNotFound)?;
    enforcement.save_ignore(actor.id(), target_id).await;
    Ok(())
}

pub async fn stop_ignoring(
    enforcement: &(impl EnforcementRepository + ?Sized),
    actor: &User,
    target_id: UserId,
) {
    enforcement.delete_ignore(actor.id(), target_id).await;
}

pub async fn ignored_by(
    enforcement: &(impl EnforcementRepository + ?Sized),
    user_id: UserId,
) -> Vec<UserId> {
    enforcement.list_ignored(user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeEnforcementRepo, FakeSessionRepo, FakeUserRepo};
    use domain::{Email, Session, SessionId, SessionToken, Username};
    use time::Duration;

    fn moderator() -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(1)),
            Username::parse("mod_user").unwrap(),
            Email::parse("m@example.com").unwrap(),
            "hash".to_owned(),
        )
        .promoted_to_moderator()
    }

    fn plain() -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(2)),
            Username::parse("plain_user").unwrap(),
            Email::parse("p@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn reason() -> Reason {
        Reason::parse("spam").unwrap()
    }

    fn repos() -> (FakeUserRepo, FakeSessionRepo, FakeEnforcementRepo) {
        let users = FakeUserRepo::with(moderator());
        (users, FakeSessionRepo::new(), FakeEnforcementRepo::new())
    }

    async fn seeded() -> (FakeUserRepo, FakeSessionRepo, FakeEnforcementRepo) {
        let (users, sessions, enforcement) = repos();
        users.save(&plain()).await;
        (users, sessions, enforcement)
    }

    #[tokio::test]
    async fn a_moderator_bans_a_user_and_ends_their_sessions() {
        let (users, sessions, enforcement) = seeded().await;
        sessions
            .save(&Session::new(
                SessionId::new(uuid::Uuid::nil()),
                plain().id(),
                SessionToken::parse(&"t".repeat(32)).unwrap(),
                OffsetDateTime::UNIX_EPOCH,
            ))
            .await;
        let ban = ban_user(
            &users,
            &sessions,
            &enforcement,
            &moderator(),
            plain().id(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        )
        .await
        .unwrap();
        assert_eq!(ban.reason(), &reason());
        assert!(
            active_ban(&enforcement, plain().id(), OffsetDateTime::UNIX_EPOCH)
                .await
                .is_some()
        );
        assert!(
            sessions
                .find_by_token(&SessionToken::parse(&"t".repeat(32)).unwrap())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_plain_user_cannot_ban_anyone() {
        let (users, sessions, enforcement) = seeded().await;
        let result = ban_user(
            &users,
            &sessions,
            &enforcement,
            &plain(),
            moderator().id(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        )
        .await;
        assert_eq!(result, Err(EnforcementError::NotAuthorized));
    }

    #[tokio::test]
    async fn a_moderator_cannot_ban_themselves() {
        let (users, sessions, enforcement) = seeded().await;
        let result = ban_user(
            &users,
            &sessions,
            &enforcement,
            &moderator(),
            moderator().id(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        )
        .await;
        assert_eq!(result, Err(EnforcementError::NotYourself));
    }

    #[tokio::test]
    async fn banning_an_unknown_user_is_refused() {
        let (users, sessions, enforcement) = repos();
        let result = ban_user(
            &users,
            &sessions,
            &enforcement,
            &moderator(),
            UserId::new(uuid::Uuid::from_u128(99)),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        )
        .await;
        assert_eq!(result, Err(EnforcementError::UserNotFound));
    }

    #[tokio::test]
    async fn a_timed_ban_stops_counting_once_it_lapses() {
        let (users, sessions, enforcement) = seeded().await;
        let start = OffsetDateTime::UNIX_EPOCH;
        ban_user(
            &users,
            &sessions,
            &enforcement,
            &moderator(),
            plain().id(),
            reason(),
            start,
            Some(start + Duration::days(1)),
        )
        .await
        .unwrap();
        assert!(active_ban(&enforcement, plain().id(), start).await.is_some());
        assert!(
            active_ban(&enforcement, plain().id(), start + Duration::days(2))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn lifting_a_ban_clears_it() {
        let (users, sessions, enforcement) = seeded().await;
        ban_user(
            &users,
            &sessions,
            &enforcement,
            &moderator(),
            plain().id(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        )
        .await
        .unwrap();
        lift_ban(&enforcement, &moderator(), plain().id())
            .await
            .unwrap();
        assert!(
            active_ban(&enforcement, plain().id(), OffsetDateTime::UNIX_EPOCH)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn only_a_moderator_lifts_a_ban() {
        let (_, _, enforcement) = repos();
        assert_eq!(
            lift_ban(&enforcement, &plain(), moderator().id()).await,
            Err(EnforcementError::NotAuthorized)
        );
    }

    #[tokio::test]
    async fn a_warning_reaches_only_its_recipient() {
        let (users, _, enforcement) = seeded().await;
        warn_user(
            &users,
            &enforcement,
            &moderator(),
            WarningId::new(uuid::Uuid::nil()),
            plain().id(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let theirs = list_warnings(&enforcement, plain().id()).await;
        assert_eq!(theirs.len(), 1);
        assert!(!theirs[0].is_acknowledged());
        assert!(list_warnings(&enforcement, moderator().id()).await.is_empty());
    }

    #[tokio::test]
    async fn a_moderator_promotes_another_user() {
        let (users, _, _) = seeded().await;
        let promoted = promote_to_moderator(&users, &moderator(), plain().id())
            .await
            .unwrap();
        assert!(promoted.role().is_moderator());
        assert!(
            users
                .find_by_id(plain().id())
                .await
                .unwrap()
                .role()
                .is_moderator()
        );
    }

    #[tokio::test]
    async fn a_plain_user_cannot_promote_anyone() {
        let (users, _, _) = seeded().await;
        let result = promote_to_moderator(&users, &plain(), plain().id()).await;
        assert_eq!(result, Err(EnforcementError::NotAuthorized));
    }

    #[tokio::test]
    async fn a_plain_user_cannot_warn() {
        let (users, _, enforcement) = seeded().await;
        let result = warn_user(
            &users,
            &enforcement,
            &plain(),
            WarningId::new(uuid::Uuid::nil()),
            moderator().id(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EnforcementError::NotAuthorized));
    }

    #[tokio::test]
    async fn acknowledging_marks_every_outstanding_warning() {
        let (users, _, enforcement) = seeded().await;
        for n in [10u128, 11] {
            warn_user(
                &users,
                &enforcement,
                &moderator(),
                WarningId::new(uuid::Uuid::from_u128(n)),
                plain().id(),
                reason(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        acknowledge_warnings(&enforcement, plain().id()).await;
        let theirs = list_warnings(&enforcement, plain().id()).await;
        assert_eq!(theirs.len(), 2);
        assert!(theirs.iter().all(|w| w.is_acknowledged()));
    }

    #[tokio::test]
    async fn ignoring_is_one_way_and_per_viewer() {
        let (users, _, enforcement) = seeded().await;
        ignore_user(&users, &enforcement, &moderator(), plain().id())
            .await
            .unwrap();
        assert_eq!(ignored_by(&enforcement, moderator().id()).await, vec![plain().id()]);
        assert!(ignored_by(&enforcement, plain().id()).await.is_empty());
    }

    #[tokio::test]
    async fn ignoring_yourself_is_refused() {
        let (users, _, enforcement) = seeded().await;
        let result = ignore_user(&users, &enforcement, &plain(), plain().id()).await;
        assert_eq!(result, Err(EnforcementError::NotYourself));
    }

    #[tokio::test]
    async fn ignoring_twice_keeps_one_entry_and_can_be_undone() {
        let (users, _, enforcement) = seeded().await;
        for _ in 0..2 {
            ignore_user(&users, &enforcement, &moderator(), plain().id())
                .await
                .unwrap();
        }
        assert_eq!(ignored_by(&enforcement, moderator().id()).await.len(), 1);
        stop_ignoring(&enforcement, &moderator(), plain().id()).await;
        assert!(ignored_by(&enforcement, moderator().id()).await.is_empty());
    }
}
