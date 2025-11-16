use crate::ports::{CommentRepository, NotificationRepository, TopicRepository};
use crate::paging::Paged;
use domain::{Body, Comment, CommentId, Notification, NotificationId, Page, TopicId, UserId};
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

pub async fn list_comments(
    comments: &(impl CommentRepository + ?Sized),
    topic_id: TopicId,
    page: Page,
) -> Paged<Comment> {
    let items = comments.list_by_topic(topic_id, page).await;
    let total = comments.count_roots(topic_id).await;
    Paged::new(items, page, total)
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
        assert_eq!(list_comments(&comments, topic().id(), Page::first()).await.items.len(), 1);
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
        let raised = notifications.list_by_recipient(topic().author_id(), Page::first()).await;
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
        let raised = notifications.list_by_recipient(parent_author, Page::first()).await;
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
                .list_by_recipient(topic().author_id(), Page::first())
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn paging_keeps_a_thread_whole() {
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        let notifications = FakeNotificationRepo::new();
        let mut roots = Vec::new();
        for n in 1u128..=3 {
            let root = post_comment(
                &topics,
                &comments,
                &notifications,
                CommentId::new(uuid::Uuid::from_u128(n)),
                NotificationId::new(uuid::Uuid::new_v4()),
                topic().id(),
                UserId::new(uuid::Uuid::nil()),
                None,
                Body::parse(&format!("Root {n}")).unwrap(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
            post_comment(
                &topics,
                &comments,
                &notifications,
                CommentId::new(uuid::Uuid::from_u128(n + 100)),
                NotificationId::new(uuid::Uuid::new_v4()),
                topic().id(),
                UserId::new(uuid::Uuid::nil()),
                Some(root.id()),
                Body::parse(&format!("Reply to {n}")).unwrap(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
            roots.push(root.id());
        }

        let first = list_comments(&comments, topic().id(), Page::parse(1, 2).unwrap()).await;
        assert_eq!(first.total, 3);
        assert_eq!(first.total_pages(), 2);
        assert!(first.has_next());
        assert_eq!(first.items.len(), 4);
        assert!(first.items.iter().any(|c| c.id() == roots[0]));
        assert!(
            first
                .items
                .iter()
                .any(|c| c.parent_id() == Some(roots[0]))
        );
        assert!(!first.items.iter().any(|c| c.id() == roots[2]));

        let second = list_comments(&comments, topic().id(), Page::parse(2, 2).unwrap()).await;
        assert_eq!(second.items.len(), 2);
        assert!(second.items.iter().any(|c| c.id() == roots[2]));
        assert!(
            second
                .items
                .iter()
                .any(|c| c.parent_id() == Some(roots[2]))
        );
        assert!(!second.has_next());
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
        assert_eq!(list_comments(&comments, topic().id(), Page::first()).await.items.len(), 2);
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
