use crate::ports::{PasswordHasher, UserRepository};
use domain::{Email, User, UserId, Username};

#[derive(Debug, PartialEq, Eq)]
pub enum RegisterError {
    UsernameTaken,
    EmailTaken,
}

pub async fn register(
    repo: &impl UserRepository,
    hasher: &impl PasswordHasher,
    new_id: UserId,
    username: Username,
    email: Email,
    plain_password: &str,
) -> Result<User, RegisterError> {
    if repo.find_by_username(&username).await.is_some() {
        return Err(RegisterError::UsernameTaken);
    }
    if repo.find_by_email(&email).await.is_some() {
        return Err(RegisterError::EmailTaken);
    }
    let password_hash = hasher.hash(plain_password);
    let mut user = User::register(new_id, username, email, password_hash);
    if repo.count().await == 0 {
        user = user.promoted_to_moderator();
    }
    repo.save(&user).await;
    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeHasher, FakeUserRepo};

    fn username(raw: &str) -> Username {
        Username::parse(raw).unwrap()
    }

    fn email(raw: &str) -> Email {
        Email::parse(raw).unwrap()
    }

    #[tokio::test]
    async fn registers_new_user() {
        let repo = FakeUserRepo::new();
        let hasher = FakeHasher;
        let id = UserId::new(uuid::Uuid::nil());
        let user = register(
            &repo,
            &hasher,
            id,
            username("alice_01"),
            email("alice@example.com"),
            "secret",
        )
        .await
        .unwrap();
        assert_eq!(user.password_hash(), "hashed:secret");
        assert!(repo.find_by_username(&username("alice_01")).await.is_some());
    }

    #[tokio::test]
    async fn first_registered_user_is_a_moderator() {
        let repo = FakeUserRepo::new();
        let hasher = FakeHasher;
        let user = register(
            &repo,
            &hasher,
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            email("alice@example.com"),
            "secret",
        )
        .await
        .unwrap();
        assert_eq!(user.role(), domain::Role::Moderator);
    }

    #[tokio::test]
    async fn later_registered_users_are_plain_users() {
        let existing = User::register(
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            email("alice@example.com"),
            "hash".to_owned(),
        );
        let repo = FakeUserRepo::with(existing);
        let hasher = FakeHasher;
        let user = register(
            &repo,
            &hasher,
            UserId::new(uuid::Uuid::max()),
            username("bob_02"),
            email("bob@example.com"),
            "secret",
        )
        .await
        .unwrap();
        assert_eq!(user.role(), domain::Role::User);
    }

    #[tokio::test]
    async fn rejects_taken_username() {
        let existing = User::register(
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            email("alice@example.com"),
            "hash".to_owned(),
        );
        let repo = FakeUserRepo::with(existing);
        let hasher = FakeHasher;
        let result = register(
            &repo,
            &hasher,
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            email("other@example.com"),
            "secret",
        )
        .await;
        assert_eq!(result, Err(RegisterError::UsernameTaken));
    }

    #[tokio::test]
    async fn rejects_taken_email() {
        let existing = User::register(
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            email("alice@example.com"),
            "hash".to_owned(),
        );
        let repo = FakeUserRepo::with(existing);
        let hasher = FakeHasher;
        let result = register(
            &repo,
            &hasher,
            UserId::new(uuid::Uuid::nil()),
            username("bob_02"),
            email("alice@example.com"),
            "secret",
        )
        .await;
        assert_eq!(result, Err(RegisterError::EmailTaken));
    }
}
