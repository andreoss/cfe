use domain::{Email, User, Username};

#[async_trait::async_trait]
pub trait UserRepository {
    async fn find_by_username(&self, username: &Username) -> Option<User>;
    async fn find_by_email(&self, email: &Email) -> Option<User>;
    async fn save(&self, user: &User);
}

pub trait PasswordHasher {
    fn hash(&self, plain: &str) -> String;
    fn verify(&self, plain: &str, hash: &str) -> bool;
}
