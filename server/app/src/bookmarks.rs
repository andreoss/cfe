use crate::ports::{BookmarkRepository, TopicRepository};
use domain::{Bookmark, Topic, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum BookmarkError {
    TopicNotFound,
}

pub async fn add_bookmark(
    topics: &(impl TopicRepository + ?Sized),
    bookmarks: &(impl BookmarkRepository + ?Sized),
    user_id: UserId,
    topic_id: TopicId,
    now: OffsetDateTime,
) -> Result<(), BookmarkError> {
    topics
        .find_by_id(topic_id)
        .await
        .ok_or(BookmarkError::TopicNotFound)?;
    bookmarks
        .save(&Bookmark::new(user_id, topic_id, now))
        .await;
    Ok(())
}

pub async fn remove_bookmark(
    bookmarks: &(impl BookmarkRepository + ?Sized),
    user_id: UserId,
    topic_id: TopicId,
) {
    bookmarks.delete(user_id, topic_id).await;
}

pub async fn list_bookmarked_topics(
    bookmarks: &(impl BookmarkRepository + ?Sized),
    user_id: UserId,
) -> Vec<Topic> {
    bookmarks.list_topics(user_id).await
}

pub async fn is_bookmarked(
    bookmarks: &(impl BookmarkRepository + ?Sized),
    user_id: UserId,
    topic_id: TopicId,
) -> bool {
    bookmarks.exists(user_id, topic_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeBookmarkRepo, FakeTopicRepo};
    use domain::{Body, SectionId, TagSet, Title};

    fn user_id() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::max()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn saves_a_topic_and_lists_it_back() {
        let topics = FakeTopicRepo::with(topic());
        let bookmarks = FakeBookmarkRepo::new();
        assert!(!is_bookmarked(&bookmarks, user_id(), topic().id()).await);
        add_bookmark(
            &topics,
            &bookmarks,
            user_id(),
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(is_bookmarked(&bookmarks, user_id(), topic().id()).await);
        let listed = list_bookmarked_topics(&bookmarks, user_id()).await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id(), topic().id());
    }

    #[tokio::test]
    async fn saving_the_same_topic_twice_keeps_one_bookmark() {
        let topics = FakeTopicRepo::with(topic());
        let bookmarks = FakeBookmarkRepo::new();
        for _ in 0..2 {
            add_bookmark(
                &topics,
                &bookmarks,
                user_id(),
                topic().id(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        assert_eq!(list_bookmarked_topics(&bookmarks, user_id()).await.len(), 1);
    }

    #[tokio::test]
    async fn rejects_saving_an_unknown_topic() {
        let topics = FakeTopicRepo::new();
        let bookmarks = FakeBookmarkRepo::new();
        let result = add_bookmark(
            &topics,
            &bookmarks,
            user_id(),
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(BookmarkError::TopicNotFound));
    }

    #[tokio::test]
    async fn removes_a_saved_topic() {
        let topics = FakeTopicRepo::with(topic());
        let bookmarks = FakeBookmarkRepo::new();
        add_bookmark(
            &topics,
            &bookmarks,
            user_id(),
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        remove_bookmark(&bookmarks, user_id(), topic().id()).await;
        assert!(!is_bookmarked(&bookmarks, user_id(), topic().id()).await);
        assert!(list_bookmarked_topics(&bookmarks, user_id()).await.is_empty());
    }

    #[tokio::test]
    async fn one_users_bookmarks_are_not_anothers() {
        let topics = FakeTopicRepo::with(topic());
        let bookmarks = FakeBookmarkRepo::new();
        add_bookmark(
            &topics,
            &bookmarks,
            user_id(),
            topic().id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let other = UserId::new(uuid::Uuid::max());
        assert!(!is_bookmarked(&bookmarks, other, topic().id()).await);
        assert!(list_bookmarked_topics(&bookmarks, other).await.is_empty());
    }
}
