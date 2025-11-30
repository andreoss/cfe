use crate::ports::{PasswordHasher, UserRepository};
use domain::{Email, User, UserId, Username};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSettings {
    pub username: String,
    pub password: String,
    pub email: String,
}

impl Default for OperatorSettings {
    fn default() -> Self {
        Self {
            username: "admin".to_owned(),
            password: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OperatorError {
    InvalidUsername,
    InvalidEmail,
}

pub async fn ensure_operator(
    users: &(impl UserRepository + ?Sized),
    hasher: &(impl PasswordHasher + ?Sized),
    settings: &OperatorSettings,
    id: UserId,
    now: OffsetDateTime,
) -> Result<Option<User>, OperatorError> {
    let username =
        Username::parse(&settings.username).map_err(|_| OperatorError::InvalidUsername)?;
    if users.find_by_username(&username).await.is_some() {
        return Ok(None);
    }
    let email = Email::parse(&settings.email).map_err(|_| OperatorError::InvalidEmail)?;
    let operator = User::register(id, username, email, hasher.hash(&settings.password))
        .promoted_to_moderator()
        .confirmed(now);
    users.save(&operator).await;
    Ok(Some(operator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeHasher, FakeUserRepo};
    use domain::Role;

    fn id() -> UserId {
        UserId::new(uuid::Uuid::from_u128(1))
    }

    fn settings() -> OperatorSettings {
        OperatorSettings::default()
    }

    async fn ensure(repo: &FakeUserRepo, settings: &OperatorSettings) -> Option<User> {
        ensure_operator(
            repo,
            &FakeHasher,
            settings,
            id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn an_empty_instance_gets_an_operator() {
        let repo = FakeUserRepo::new();
        let operator = ensure(&repo, &settings()).await.unwrap();
        assert_eq!(operator.username().as_str(), "admin");
        assert_eq!(operator.role(), Role::Moderator);
    }

    #[tokio::test]
    async fn the_operator_is_confirmed_so_housekeeping_leaves_it_alone() {
        let repo = FakeUserRepo::new();
        let operator = ensure(&repo, &settings()).await.unwrap();
        assert!(operator.is_confirmed());
    }

    #[tokio::test]
    async fn its_password_is_the_one_configured() {
        let repo = FakeUserRepo::new();
        let operator = ensure(&repo, &settings()).await.unwrap();
        assert!(FakeHasher.verify("admin", operator.password_hash()));
    }

    #[tokio::test]
    async fn starting_again_leaves_the_existing_one_alone() {
        let repo = FakeUserRepo::new();
        ensure(&repo, &settings()).await.unwrap();
        assert_eq!(ensure(&repo, &settings()).await, None);
        assert_eq!(repo.count().await, 1);
    }

    #[tokio::test]
    async fn a_changed_password_does_not_overwrite_the_account() {
        let repo = FakeUserRepo::new();
        ensure(&repo, &settings()).await.unwrap();
        let changed = OperatorSettings {
            password: "something-else".to_owned(),
            ..settings()
        };
        assert_eq!(ensure(&repo, &changed).await, None);
        let stored = repo
            .find_by_username(&Username::parse("admin").unwrap())
            .await
            .unwrap();
        assert!(FakeHasher.verify("admin", stored.password_hash()));
    }

    #[tokio::test]
    async fn the_name_and_password_come_from_configuration() {
        let repo = FakeUserRepo::new();
        let chosen = OperatorSettings {
            username: "keeper".to_owned(),
            password: "a longer secret".to_owned(),
            email: "keeper@example.com".to_owned(),
        };
        let operator = ensure(&repo, &chosen).await.unwrap();
        assert_eq!(operator.username().as_str(), "keeper");
        assert!(FakeHasher.verify("a longer secret", operator.password_hash()));
    }

    #[tokio::test]
    async fn an_operator_may_join_an_instance_that_already_has_accounts() {
        let repo = FakeUserRepo::new();
        let existing = User::register(
            UserId::new(uuid::Uuid::from_u128(9)),
            Username::parse("someone_01").unwrap(),
            Email::parse("someone@example.com").unwrap(),
            "hash".to_owned(),
        );
        repo.save(&existing).await;
        let operator = ensure(&repo, &settings()).await.unwrap();
        assert_eq!(operator.role(), Role::Moderator);
        assert_eq!(repo.count().await, 2);
    }

    #[tokio::test]
    async fn a_name_no_account_could_have_is_refused() {
        let repo = FakeUserRepo::new();
        let bad = OperatorSettings {
            username: "x".to_owned(),
            ..settings()
        };
        assert_eq!(
            ensure_operator(&repo, &FakeHasher, &bad, id(), OffsetDateTime::UNIX_EPOCH).await,
            Err(OperatorError::InvalidUsername)
        );
    }

    #[tokio::test]
    async fn an_address_no_account_could_have_is_refused() {
        let repo = FakeUserRepo::new();
        let bad = OperatorSettings {
            email: "not-an-address".to_owned(),
            ..settings()
        };
        assert_eq!(
            ensure_operator(&repo, &FakeHasher, &bad, id(), OffsetDateTime::UNIX_EPOCH).await,
            Err(OperatorError::InvalidEmail)
        );
    }

    #[tokio::test]
    async fn the_password_is_not_held_in_the_clear() {
        let repo = FakeUserRepo::new();
        let operator = ensure(&repo, &settings()).await.unwrap();
        assert_ne!(operator.password_hash(), "admin");
    }
}
