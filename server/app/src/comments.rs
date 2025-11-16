use crate::ports::{CommentRepository, NotificationRepository, TopicRepository};
use domain::{Body, Comment, CommentId, Notification, NotificationId, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum PostCommentError {
    TopicNotFound,
    ParentNotFound,
    ParentInDifferentTopic,
}

pub async fn post_comment(
    topics: &(impl TopicRepository + ?Sized),
    comments: &(impl CommentRepository + ?Sized),
    notifications: &(impl NotificationRepository + ?Sized),
    id: CommentId,
    notification_id: NotificationId,
    topic_id: TopicId,
    author_id: UserId,
    parent_id: Option<CommentId>,
    body: Body,
    now: OffsetDateTime,
) -> Result<Comment, PostCommentError> {
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(PostCommentError::TopicNotFound)?;
    let mut recipient_id = topic.author_id();
    if let Some(parent_id) = parent_id {
        let parent = comments
            .find_by_id(parent_id)
            .await
            .ok_or(PostCommentError::ParentNotFound)?;
        if parent.topic_id() != topic_id {
            return Err(PostCommentError::ParentInDifferentTopic);
        }
        recipient_id = parent.author_id();
    }
    let comment = Comment::new(id, topic_id, author_id, parent_id, body, now);
    comments.save(&comment).await;
    if recipient_id != author_id {
        notifications
            .save(&Notification::new(
                notification_id,
                recipient_id,
                author_id,
                topic_id,
                id,
                now,
            ))
            .await;
    }
    Ok(comment)
}

pub async fn list_comments(comments: &(impl CommentRepository + ?Sized), topic_id: TopicId) -> Vec<Comment> {
    comments.list_by_topic(topic_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeNotificationRepo, FakeTopicRepo};
    use domain::{SectionId, TagSet, Title, Topic};

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn posts_a_top_level_comment() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let comment = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Nice topic!").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(comment.parent_id(), None);
        assert_eq!(list_comments(&comments, topic().id()).await.len(), 1);
    }

    #[tokio::test]
    async fn notifies_the_topic_author_of_a_new_comment() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let commenter = UserId::new(uuid::Uuid::max());
        post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            commenter,
            None,
            Body::parse("Nice topic!").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let raised = notifications.list_by_recipient(topic().author_id()).await;
        assert_eq!(raised.len(), 1);
        assert_eq!(raised[0].actor_id(), commenter);
        assert_eq!(raised[0].topic_id(), topic().id());
        assert!(!raised[0].is_read());
    }

    #[tokio::test]
    async fn notifies_the_parent_author_of_a_reply() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let parent_author = UserId::new(uuid::Uuid::from_u128(7));
        let replier = UserId::new(uuid::Uuid::max());
        let root = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            parent_author,
            None,
            Body::parse("Root").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::max()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            replier,
            Some(root.id()),
            Body::parse("Reply").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let raised = notifications.list_by_recipient(parent_author).await;
        assert_eq!(raised.len(), 1);
        assert_eq!(raised[0].actor_id(), replier);
        assert_eq!(raised[0].comment_id(), CommentId::new(uuid::Uuid::max()));
    }

    #[tokio::test]
    async fn does_not_notify_you_about_your_own_comment() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            topic().author_id(),
            None,
            Body::parse("Replying to myself").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(
            notifications
                .list_by_recipient(topic().author_id())
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn rejects_posting_to_an_unknown_topic() {
        let topics = FakeTopicRepo::new();
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let result = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Nice topic!").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(PostCommentError::TopicNotFound));
    }

    #[tokio::test]
    async fn posts_a_reply_to_an_existing_comment() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let root = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Root").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let reply = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::max()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            UserId::new(uuid::Uuid::nil()),
            Some(root.id()),
            Body::parse("Reply").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(reply.parent_id(), Some(root.id()));
        assert_eq!(list_comments(&comments, topic().id()).await.len(), 2);
    }

    #[tokio::test]
    async fn rejects_replying_to_an_unknown_parent() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let result = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            UserId::new(uuid::Uuid::nil()),
            Some(CommentId::new(uuid::Uuid::max())),
            Body::parse("Reply").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(PostCommentError::ParentNotFound));
    }

    #[tokio::test]
    async fn rejects_a_parent_from_a_different_topic() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let other_topic_id = TopicId::new(uuid::Uuid::max());
        let foreign_parent = Comment::new(
            CommentId::new(uuid::Uuid::max()),
            other_topic_id,
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Elsewhere").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        comments.save(&foreign_parent).await;
        let result = post_comment(
            &topics,
            &comments,
            &notifications,
            CommentId::new(uuid::Uuid::nil()),
            NotificationId::new(uuid::Uuid::new_v4()),
            topic().id(),
            UserId::new(uuid::Uuid::nil()),
            Some(foreign_parent.id()),
            Body::parse("Reply").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(PostCommentError::ParentInDifferentTopic));
    }
}
