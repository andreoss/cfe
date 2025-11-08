use crate::ports::UserRepository;
use domain::{Bio, User, UserId};

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateBioError {
    NotFound,
}

pub async fn update_bio(
    repo: &impl UserRepository,
    user_id: UserId,
    bio: Option<Bio>,
) -> Result<User, UpdateBioError> {
    let user = repo
        .find_by_id(user_id)
        .await
        .ok_or(UpdateBioError::NotFound)?;
    let updated = user.with_bio(bio);
    repo.update(&updated).await;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeUserRepo;
    use domain::{Email, Username};

    fn user() -> User {
        User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("alice_01").unwrap(),
            Email::parse("alice@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    #[tokio::test]
    async fn sets_the_bio_on_an_existing_user() {
        let repo = FakeUserRepo::with(user());
        let bio = Bio::parse("hello").unwrap();
        let updated = update_bio(&repo, UserId::new(uuid::Uuid::nil()), bio.clone())
            .await
            .unwrap();
        assert_eq!(updated.bio(), bio.as_ref());
        let stored = repo
            .find_by_id(UserId::new(uuid::Uuid::nil()))
            .await
            .unwrap();
        assert_eq!(stored.bio(), bio.as_ref());
    }

    #[tokio::test]
    async fn clears_the_bio_when_given_none() {
        let repo = FakeUserRepo::with(user().with_bio(Bio::parse("old").unwrap()));
        let updated = update_bio(&repo, UserId::new(uuid::Uuid::nil()), None)
            .await
            .unwrap();
        assert_eq!(updated.bio(), None);
    }

    #[tokio::test]
    async fn rejects_unknown_user() {
        let repo = FakeUserRepo::new();
        let result = update_bio(&repo, UserId::new(uuid::Uuid::nil()), None).await;
        assert_eq!(result, Err(UpdateBioError::NotFound));
    }
}
