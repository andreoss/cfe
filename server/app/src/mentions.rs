use crate::ports::{NotificationRepository, UserRepository};
use domain::{
    CommentId, Notification, NotificationId, NotificationKind, TopicId, UserId, mentioned_in,
};
use time::OffsetDateTime;

#[allow(clippy::too_many_arguments)]
pub async fn notify_mentioned(
    users: &(impl UserRepository + ?Sized),
    notifications: &(impl NotificationRepository + ?Sized),
    body: &str,
    topic_id: TopicId,
    comment_id: CommentId,
    actor_id: UserId,
    already_told: &[UserId],
    mut next_id: impl FnMut() -> NotificationId,
    now: OffsetDateTime,
) -> usize {
    let mut told = 0;
    for name in mentioned_in(body) {
        let Some(user) = users.find_by_username(&name).await else {
            continue;
        };
        if user.id() == actor_id || already_told.contains(&user.id()) {
            continue;
        }
        notifications
            .save(
                &Notification::new(next_id(), user.id(), actor_id, topic_id, comment_id, now)
                    .of_kind(NotificationKind::Mention),
            )
            .await;
        told += 1;
    }
    told
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeNotificationRepo, FakeUserRepo};
    use domain::{Email, Page, User, Username};

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn topic_id() -> TopicId {
        TopicId::new(uuid::Uuid::from_u128(50))
    }

    fn comment_id() -> CommentId {
        CommentId::new(uuid::Uuid::from_u128(60))
    }

    async fn seeded() -> (FakeUserRepo, FakeNotificationRepo) {
        let users = FakeUserRepo::new();
        users.save(&user(1, "alice_01")).await;
        users.save(&user(2, "bob_02")).await;
        users.save(&user(3, "carla_03")).await;
        (users, FakeNotificationRepo::new())
    }

    fn ids() -> impl FnMut() -> NotificationId {
        let mut n = 100u128;
        move || {
            n += 1;
            NotificationId::new(uuid::Uuid::from_u128(n))
        }
    }

    async fn told_to(notifications: &FakeNotificationRepo, id: UserId) -> usize {
        notifications
            .list_by_recipient(id, Page::first())
            .await
            .len()
    }

    #[tokio::test]
    async fn somebody_named_is_told() {
        let (users, notifications) = seeded().await;
        let told = notify_mentioned(
            &users,
            &notifications,
            "I agree with @alice_01 on this",
            topic_id(),
            comment_id(),
            user(2, "bob_02").id(),
            &[],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 1);
        assert_eq!(told_to(&notifications, user(1, "alice_01").id()).await, 1);
    }

    #[tokio::test]
    async fn the_notice_says_it_was_a_mention() {
        let (users, notifications) = seeded().await;
        notify_mentioned(
            &users,
            &notifications,
            "@alice_01",
            topic_id(),
            comment_id(),
            user(2, "bob_02").id(),
            &[],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        let listed = notifications
            .list_by_recipient(user(1, "alice_01").id(), Page::first())
            .await;
        assert_eq!(listed[0].kind(), NotificationKind::Mention);
    }

    #[tokio::test]
    async fn naming_yourself_tells_you_nothing() {
        let (users, notifications) = seeded().await;
        let told = notify_mentioned(
            &users,
            &notifications,
            "as @alice_01 already said",
            topic_id(),
            comment_id(),
            user(1, "alice_01").id(),
            &[],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 0);
    }

    #[tokio::test]
    async fn somebody_already_told_is_not_told_twice() {
        let (users, notifications) = seeded().await;
        let told = notify_mentioned(
            &users,
            &notifications,
            "@alice_01 and @bob_02",
            topic_id(),
            comment_id(),
            user(3, "carla_03").id(),
            &[user(1, "alice_01").id()],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 1);
        assert_eq!(told_to(&notifications, user(1, "alice_01").id()).await, 0);
        assert_eq!(told_to(&notifications, user(2, "bob_02").id()).await, 1);
    }

    #[tokio::test]
    async fn a_name_nobody_answers_to_is_passed_over() {
        let (users, notifications) = seeded().await;
        let told = notify_mentioned(
            &users,
            &notifications,
            "@nobody_here and @alice_01",
            topic_id(),
            comment_id(),
            user(2, "bob_02").id(),
            &[],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 1);
    }

    #[tokio::test]
    async fn naming_somebody_twice_tells_them_once() {
        let (users, notifications) = seeded().await;
        let told = notify_mentioned(
            &users,
            &notifications,
            "@alice_01 @alice_01",
            topic_id(),
            comment_id(),
            user(2, "bob_02").id(),
            &[],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 1);
    }

    #[tokio::test]
    async fn a_body_naming_nobody_tells_nobody() {
        let (users, notifications) = seeded().await;
        let told = notify_mentioned(
            &users,
            &notifications,
            "no names here at all",
            topic_id(),
            comment_id(),
            user(2, "bob_02").id(),
            &[],
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 0);
    }
}
