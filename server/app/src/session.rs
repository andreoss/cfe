use crate::ports::{SessionRepository, UserRepository};
use domain::{Session, SessionId, SessionToken, User};
use time::OffsetDateTime;

pub async fn create_session(repo: &(impl SessionRepository + ?Sized), session: Session) -> Session {
    repo.save(&session).await;
    session
}

pub async fn current_user(
    sessions: &(impl SessionRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    token: &SessionToken,
    now: OffsetDateTime,
) -> Option<User> {
    let session = sessions.find_by_token(token).await?;
    if !session.is_valid_at(now) {
        return None;
    }
    users
        .find_by_id(session.user_id())
        .await
        .filter(|u| u.is_active())
}

pub async fn touch_session(
    repo: &(impl SessionRepository + ?Sized),
    id: SessionId,
    new_expiry: OffsetDateTime,
) {
    repo.touch(id, new_expiry).await;
}

pub async fn sign_out(repo: &(impl SessionRepository + ?Sized), id: SessionId) {
    repo.delete(id).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeSessionRepo, FakeUserRepo};
    use domain::{Email, SessionId as DomainSessionId, UserId, Username};
    use time::Duration;

    fn user() -> User {
        User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("alice_01").unwrap(),
            Email::parse("alice@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn token() -> SessionToken {
        SessionToken::parse("0123456789abcdef").unwrap()
    }

    #[tokio::test]
    async fn current_user_returns_none_for_unknown_token() {
        let sessions = FakeSessionRepo::new();
        let users = FakeUserRepo::with(user());
        let now = OffsetDateTime::UNIX_EPOCH;
        let result = current_user(&sessions, &users, &token(), now).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn current_user_returns_none_for_expired_session() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let session = Session::new(
            DomainSessionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            token(),
            now - Duration::seconds(1),
        );
        let sessions = FakeSessionRepo::with(session);
        let users = FakeUserRepo::with(user());
        let result = current_user(&sessions, &users, &token(), now).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn current_user_returns_user_for_valid_session() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let session = Session::new(
            DomainSessionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            token(),
            now + Duration::days(1),
        );
        let sessions = FakeSessionRepo::with(session);
        let users = FakeUserRepo::with(user());
        let result = current_user(&sessions, &users, &token(), now)
            .await
            .unwrap();
        assert_eq!(result.username(), user().username());
    }

    #[tokio::test]
    async fn current_user_returns_none_for_a_deregistered_account() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let session = Session::new(
            DomainSessionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            token(),
            now + Duration::days(1),
        );
        let sessions = FakeSessionRepo::with(session);
        let users = FakeUserRepo::with(user().deregistered(now));
        assert!(
            current_user(&sessions, &users, &token(), now)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn sign_out_removes_the_session() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let id = DomainSessionId::new(uuid::Uuid::nil());
        let session = Session::new(
            id,
            UserId::new(uuid::Uuid::nil()),
            token(),
            now + Duration::days(1),
        );
        let sessions = FakeSessionRepo::with(session);
        sign_out(&sessions, id).await;
        assert!(sessions.find_by_token(&token()).await.is_none());
    }
}
