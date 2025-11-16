use crate::ports::{PasswordHasher, SessionRepository, UserRepository};
use domain::{Password, User};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum ChangePasswordError {
    WrongPassword,
}

pub async fn change_password(
    users: &impl UserRepository,
    sessions: &impl SessionRepository,
    hasher: &impl PasswordHasher,
    user: &User,
    current: &str,
    new: Password,
) -> Result<User, ChangePasswordError> {
    if !hasher.verify(current, user.password_hash()) {
        return Err(ChangePasswordError::WrongPassword);
    }
    let updated = user.with_password_hash(hasher.hash(new.as_str()));
    users.update(&updated).await;
    sessions.delete_for_user(updated.id()).await;
    Ok(updated)
}

pub async fn deregister(
    users: &impl UserRepository,
    sessions: &impl SessionRepository,
    user: &User,
    now: OffsetDateTime,
) -> User {
    let gone = user.deregistered(now);
    users.update(&gone).await;
    sessions.delete_for_user(gone.id()).await;
    gone
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeHasher, FakeSessionRepo, FakeUserRepo};
    use domain::{Email, Session, SessionId, SessionToken, UserId, Username};

    fn user() -> User {
        User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("acct_owner").unwrap(),
            Email::parse("acct@example.com").unwrap(),
            "hashed:correcthorse".to_owned(),
        )
    }

    fn session_for(user: &User) -> Session {
        Session::new(
            SessionId::new(uuid::Uuid::nil()),
            user.id(),
            SessionToken::parse(&"t".repeat(32)).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn changes_the_password_when_the_current_one_matches() {
        let users = FakeUserRepo::with(user());
        let sessions = FakeSessionRepo::new();
        let updated = change_password(
            &users,
            &sessions,
            &FakeHasher,
            &user(),
            "correcthorse",
            Password::parse("newpassword").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(updated.password_hash(), "hashed:newpassword");
        assert_eq!(
            users.find_by_id(user().id()).await.unwrap().password_hash(),
            "hashed:newpassword"
        );
    }

    #[tokio::test]
    async fn refuses_a_wrong_current_password_and_changes_nothing() {
        let users = FakeUserRepo::with(user());
        let sessions = FakeSessionRepo::new();
        let result = change_password(
            &users,
            &sessions,
            &FakeHasher,
            &user(),
            "notitatall",
            Password::parse("newpassword").unwrap(),
        )
        .await;
        assert_eq!(result, Err(ChangePasswordError::WrongPassword));
        assert_eq!(
            users.find_by_id(user().id()).await.unwrap().password_hash(),
            "hashed:correcthorse"
        );
    }

    #[tokio::test]
    async fn changing_the_password_ends_existing_sessions() {
        let users = FakeUserRepo::with(user());
        let sessions = FakeSessionRepo::with(session_for(&user()));
        change_password(
            &users,
            &sessions,
            &FakeHasher,
            &user(),
            "correcthorse",
            Password::parse("newpassword").unwrap(),
        )
        .await
        .unwrap();
        assert!(
            sessions
                .find_by_token(session_for(&user()).token())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn deregistering_marks_the_account_and_ends_its_sessions() {
        let users = FakeUserRepo::with(user());
        let sessions = FakeSessionRepo::with(session_for(&user()));
        let gone = deregister(&users, &sessions, &user(), OffsetDateTime::UNIX_EPOCH).await;
        assert!(!gone.is_active());
        assert!(
            !users
                .find_by_id(user().id())
                .await
                .unwrap()
                .is_active()
        );
        assert!(
            sessions
                .find_by_token(session_for(&user()).token())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_deregistered_account_keeps_its_username() {
        let users = FakeUserRepo::with(user());
        let sessions = FakeSessionRepo::new();
        let gone = deregister(&users, &sessions, &user(), OffsetDateTime::UNIX_EPOCH).await;
        assert_eq!(gone.username(), user().username());
    }
}
