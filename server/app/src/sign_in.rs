use crate::ports::{PasswordHasher, UserRepository};
use domain::{User, Username};

#[derive(Debug, PartialEq, Eq)]
pub enum SignInError {
    NotFound,
    WrongPassword,
}

pub async fn sign_in(
    repo: &impl UserRepository,
    hasher: &impl PasswordHasher,
    username: &Username,
    plain_password: &str,
) -> Result<User, SignInError> {
    let user = repo
        .find_by_username(username)
        .await
        .filter(|u| u.is_active())
        .ok_or(SignInError::NotFound)?;
    if hasher.verify(plain_password, user.password_hash()) {
        Ok(user)
    } else {
        Err(SignInError::WrongPassword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeHasher, FakeUserRepo};
    use domain::{Email, UserId};
    use time::OffsetDateTime;

    fn username(raw: &str) -> Username {
        Username::parse(raw).unwrap()
    }

    fn seed_user() -> User {
        User::register(
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            Email::parse("alice@example.com").unwrap(),
            "hashed:secret".to_owned(),
        )
    }

    #[tokio::test]
    async fn signs_in_with_correct_password() {
        let repo = FakeUserRepo::with(seed_user());
        let hasher = FakeHasher;
        let user = sign_in(&repo, &hasher, &username("alice_01"), "secret")
            .await
            .unwrap();
        assert_eq!(user.username(), &username("alice_01"));
    }

    #[tokio::test]
    async fn rejects_wrong_password() {
        let repo = FakeUserRepo::with(seed_user());
        let hasher = FakeHasher;
        let result = sign_in(&repo, &hasher, &username("alice_01"), "wrong").await;
        assert_eq!(result, Err(SignInError::WrongPassword));
    }

    #[tokio::test]
    async fn rejects_unknown_user() {
        let repo = FakeUserRepo::new();
        let hasher = FakeHasher;
        let result = sign_in(&repo, &hasher, &username("ghost_01"), "secret").await;
        assert_eq!(result, Err(SignInError::NotFound));
    }

    #[tokio::test]
    async fn refuses_a_deregistered_account_as_if_it_did_not_exist() {
        let gone = seed_user().deregistered(OffsetDateTime::UNIX_EPOCH);
        let repo = FakeUserRepo::with(gone);
        let result = sign_in(&repo, &FakeHasher, &username("alice_01"), "secret").await;
        assert_eq!(result, Err(SignInError::NotFound));
    }
}
