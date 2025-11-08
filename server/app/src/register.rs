use domain::{Email, User, UserId, Username};

#[derive(Debug, PartialEq, Eq)]
pub enum RegisterError {
    UsernameTaken,
    EmailTaken,
}

#[async_trait::async_trait]
pub trait UserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User>;
    async fn find_by_email(&self, email: &Email) -> Option<User>;
    async fn save(&self, user: &User);
}

pub trait PasswordHasher {
    fn hash(&self, plain: &str) -> String;
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
    let user = User::register(new_id, username, email, password_hash);
    repo.save(&user).await;
    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeRepo {
        users: Mutex<Vec<User>>,
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                users: Mutex::new(Vec::new()),
            }
        }

        fn with(user: User) -> Self {
            Self {
                users: Mutex::new(vec![user]),
            }
        }
    }

    #[async_trait::async_trait]
    impl UserRepository for FakeRepo {
        async fn find_by_username(&self, username: &Username) -> Option<User> {
            self.users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.username() == username)
                .cloned()
        }

        async fn find_by_email(&self, email: &Email) -> Option<User> {
            self.users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.email() == email)
                .cloned()
        }

        async fn save(&self, user: &User) {
            self.users.lock().unwrap().push(user.clone());
        }
    }

    struct FakeHasher;

    impl PasswordHasher for FakeHasher {
        fn hash(&self, plain: &str) -> String {
            format!("hashed:{plain}")
        }
    }

    fn username(raw: &str) -> Username {
        Username::parse(raw).unwrap()
    }

    fn email(raw: &str) -> Email {
        Email::parse(raw).unwrap()
    }

    #[tokio::test]
    async fn registers_new_user() {
        let repo = FakeRepo::new();
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
    async fn rejects_taken_username() {
        let existing = User::register(
            UserId::new(uuid::Uuid::nil()),
            username("alice_01"),
            email("alice@example.com"),
            "hash".to_owned(),
        );
        let repo = FakeRepo::with(existing);
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
        let repo = FakeRepo::with(existing);
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
