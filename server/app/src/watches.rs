use crate::paging::Paged;
use crate::ports::{NotificationRepository, TopicRepository, WatchRepository};
use domain::{
    CommentId, Notification, NotificationId, NotificationKind, Page, Topic, TopicId, User, UserId,
    Watch,
};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum WatchError {
    TopicNotFound,
}

pub async fn watch_topic(
    watches: &(impl WatchRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    user: &User,
    topic_id: TopicId,
    now: OffsetDateTime,
) -> Result<(), WatchError> {
    topics
        .find_by_id(topic_id)
        .await
        .ok_or(WatchError::TopicNotFound)?;
    watches.save(&Watch::new(user.id(), topic_id, now)).await;
    Ok(())
}

pub async fn stop_watching(
    watches: &(impl WatchRepository + ?Sized),
    user: &User,
    topic_id: TopicId,
) {
    watches.delete(user.id(), topic_id).await;
}

pub async fn is_watching(
    watches: &(impl WatchRepository + ?Sized),
    user_id: UserId,
    topic_id: TopicId,
) -> bool {
    watches.find(user_id, topic_id).await.is_some()
}

pub async fn list_watched(
    watches: &(impl WatchRepository + ?Sized),
    user_id: UserId,
    page: Page,
) -> Paged<Topic> {
    let items = watches.list_topics(user_id, page).await;
    let total = watches.count_topics(user_id).await;
    Paged::new(items, page, total)
}

pub async fn notify_watchers(
    watches: &(impl WatchRepository + ?Sized),
    notifications: &(impl NotificationRepository + ?Sized),
    topic_id: TopicId,
    comment_id: CommentId,
    actor_id: UserId,
    already_told: UserId,
    mut next_id: impl FnMut() -> NotificationId,
    now: OffsetDateTime,
) -> usize {
    let mut told = 0;
    for watcher in watches.watchers(topic_id).await {
        if watcher == actor_id || watcher == already_told {
            continue;
        }
        notifications
            .save(
                &Notification::new(next_id(), watcher, actor_id, topic_id, comment_id, now)
                    .of_kind(NotificationKind::Watch),
            )
            .await;
        told += 1;
    }
    told
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeNotificationRepo, FakeTopicRepo, FakeWatchRepo};
    use domain::{Body, Email, SectionId, TagSet, Title, Username};

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::from_u128(9)),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::from_u128(1)),
            Title::parse("Watched").unwrap(),
            Body::parse("A watched topic").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn comment_id() -> CommentId {
        CommentId::new(uuid::Uuid::from_u128(50))
    }

    fn ids() -> impl FnMut() -> NotificationId {
        let mut n = 100u128;
        move || {
            n += 1;
            NotificationId::new(uuid::Uuid::from_u128(n))
        }
    }

    #[tokio::test]
    async fn watching_a_topic_records_it() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let watcher = user(2, "watcher_01");
        watch_topic(
            &watches,
            &topics,
            &watcher,
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(is_watching(&watches, watcher.id(), topic().id()).await);
    }

    #[tokio::test]
    async fn watching_twice_leaves_one_watch() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let watcher = user(2, "watcher_01");
        for _ in 0..2 {
            watch_topic(
                &watches,
                &topics,
                &watcher,
                topic().id(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        assert_eq!(watches.watchers(topic().id()).await.len(), 1);
    }

    #[tokio::test]
    async fn watching_an_unknown_topic_is_refused() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::new();
        let result = watch_topic(
            &watches,
            &topics,
            &user(2, "watcher_01"),
            TopicId::new(uuid::Uuid::from_u128(404)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(WatchError::TopicNotFound));
    }

    #[tokio::test]
    async fn a_watch_can_be_dropped_again() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let watcher = user(2, "watcher_01");
        watch_topic(
            &watches,
            &topics,
            &watcher,
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        stop_watching(&watches, &watcher, topic().id()).await;
        assert!(!is_watching(&watches, watcher.id(), topic().id()).await);
    }

    #[tokio::test]
    async fn a_new_comment_tells_the_watchers() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let notifications = FakeNotificationRepo::new();
        let first = user(2, "watcher_01");
        let second = user(3, "watcher_02");
        for watcher in [&first, &second] {
            watch_topic(
                &watches,
                &topics,
                watcher,
                topic().id(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        let told = notify_watchers(
            &watches,
            &notifications,
            topic().id(),
            comment_id(),
            user(4, "poster_01").id(),
            UserId::new(uuid::Uuid::from_u128(999)),
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 2);
        let listed = notifications
            .list_by_recipient(first.id(), Page::first())
            .await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind(), NotificationKind::Watch);
    }

    #[tokio::test]
    async fn a_watcher_is_not_told_about_their_own_comment() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let notifications = FakeNotificationRepo::new();
        let watcher = user(2, "watcher_01");
        watch_topic(
            &watches,
            &topics,
            &watcher,
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let told = notify_watchers(
            &watches,
            &notifications,
            topic().id(),
            comment_id(),
            watcher.id(),
            UserId::new(uuid::Uuid::from_u128(999)),
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 0);
    }

    #[tokio::test]
    async fn a_watcher_already_told_as_a_reply_is_not_told_twice() {
        let watches = FakeWatchRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let notifications = FakeNotificationRepo::new();
        let watcher = user(2, "watcher_01");
        watch_topic(
            &watches,
            &topics,
            &watcher,
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let told = notify_watchers(
            &watches,
            &notifications,
            topic().id(),
            comment_id(),
            user(4, "poster_01").id(),
            watcher.id(),
            ids(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(told, 0);
    }

    #[tokio::test]
    async fn watched_topics_are_listed_for_their_watcher() {
        let watches = FakeWatchRepo::new();
        watches.hold(topic());
        let topics = FakeTopicRepo::with(topic());
        let watcher = user(2, "watcher_01");
        watch_topic(
            &watches,
            &topics,
            &watcher,
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let listed = list_watched(&watches, watcher.id(), Page::first()).await;
        assert_eq!(listed.total, 1);
        assert_eq!(listed.items[0].id(), topic().id());
        let other = list_watched(&watches, user(3, "watcher_02").id(), Page::first()).await;
        assert_eq!(other.total, 0);
    }
}
