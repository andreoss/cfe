use crate::ports::{AvatarRepository, UserRepository};
use domain::{Avatar, Username};

#[derive(Debug, PartialEq, Eq)]
pub enum AvatarLookupError {
    UserNotFound,
    NoAvatar,
}

pub async fn set_avatar(avatars: &impl AvatarRepository, user_id: domain::UserId, avatar: Avatar) {
    avatars.save(user_id, &avatar).await;
}

pub async fn clear_avatar(avatars: &impl AvatarRepository, user_id: domain::UserId) {
    avatars.delete(user_id).await;
}

pub async fn get_avatar(
    users: &impl UserRepository,
    avatars: &impl AvatarRepository,
    username: &Username,
) -> Result<Avatar, AvatarLookupError> {
    let user = users
        .find_by_username(username)
        .await
        .ok_or(AvatarLookupError::UserNotFound)?;
    avatars
        .find_by_user(user.id())
        .await
        .ok_or(AvatarLookupError::NoAvatar)
}

pub async fn has_avatar(avatars: &impl AvatarRepository, user_id: domain::UserId) -> bool {
    avatars.find_by_user(user_id).await.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeAvatarRepo, FakeUserRepo};
    use domain::{Email, ImageFormat, User, UserId};

    fn png() -> Avatar {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(b"body");
        Avatar::parse(bytes).unwrap()
    }

    fn user() -> User {
        User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("pic_owner").unwrap(),
            Email::parse("p@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    #[tokio::test]
    async fn stores_and_returns_an_avatar() {
        let users = FakeUserRepo::with(user());
        let avatars = FakeAvatarRepo::new();
        assert!(!has_avatar(&avatars, user().id()).await);
        set_avatar(&avatars, user().id(), png()).await;
        assert!(has_avatar(&avatars, user().id()).await);
        let found = get_avatar(&users, &avatars, user().username()).await.unwrap();
        assert_eq!(found.format(), ImageFormat::Png);
        assert_eq!(found.bytes(), png().bytes());
    }

    #[tokio::test]
    async fn replacing_an_avatar_keeps_one_per_user() {
        let users = FakeUserRepo::with(user());
        let avatars = FakeAvatarRepo::new();
        set_avatar(&avatars, user().id(), png()).await;
        let gif = Avatar::parse(b"GIF89a....".to_vec()).unwrap();
        set_avatar(&avatars, user().id(), gif).await;
        let found = get_avatar(&users, &avatars, user().username()).await.unwrap();
        assert_eq!(found.format(), ImageFormat::Gif);
    }

    #[tokio::test]
    async fn clearing_removes_it() {
        let users = FakeUserRepo::with(user());
        let avatars = FakeAvatarRepo::new();
        set_avatar(&avatars, user().id(), png()).await;
        clear_avatar(&avatars, user().id()).await;
        assert!(!has_avatar(&avatars, user().id()).await);
        assert_eq!(
            get_avatar(&users, &avatars, user().username()).await,
            Err(AvatarLookupError::NoAvatar)
        );
    }

    #[tokio::test]
    async fn reports_a_user_without_an_avatar_apart_from_an_unknown_user() {
        let users = FakeUserRepo::with(user());
        let avatars = FakeAvatarRepo::new();
        assert_eq!(
            get_avatar(&users, &avatars, user().username()).await,
            Err(AvatarLookupError::NoAvatar)
        );
        let ghost = Username::parse("nobody_here").unwrap();
        assert_eq!(
            get_avatar(&users, &avatars, &ghost).await,
            Err(AvatarLookupError::UserNotFound)
        );
    }

    #[tokio::test]
    async fn one_users_avatar_is_not_anothers() {
        let other = User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("other_user").unwrap(),
            Email::parse("o@example.com").unwrap(),
            "hash".to_owned(),
        );
        let users = FakeUserRepo::with(user());
        users.save(&other).await;
        let avatars = FakeAvatarRepo::new();
        set_avatar(&avatars, user().id(), png()).await;
        assert!(!has_avatar(&avatars, other.id()).await);
        assert_eq!(
            get_avatar(&users, &avatars, other.username()).await,
            Err(AvatarLookupError::NoAvatar)
        );
    }
}
