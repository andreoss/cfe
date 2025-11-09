use crate::ports::{CommentRepository, TopicRepository};
use domain::{Body, Comment, CommentId, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum PostCommentError {
    TopicNotFound,
    ParentNotFound,
    ParentInDifferentTopic,
}

pub async fn post_comment(
    topics: &impl TopicRepository,
    comments: &impl CommentRepository,
    id: CommentId,
    topic_id: TopicId,
    author_id: UserId,
    parent_id: Option<CommentId>,
    body: Body,
    now: OffsetDateTime,
) -> Result<Comment, PostCommentError> {
    topics
        .find_by_id(topic_id)
        .await
        .ok_or(PostCommentError::TopicNotFound)?;
    if let Some(parent_id) = parent_id {
        let parent = comments
            .find_by_id(parent_id)
            .await
            .ok_or(PostCommentError::ParentNotFound)?;
        if parent.topic_id() != topic_id {
            return Err(PostCommentError::ParentInDifferentTopic);
        }
    }
    let comment = Comment::new(id, topic_id, author_id, parent_id, body, now);
    comments.save(&comment).await;
    Ok(comment)
}

pub async fn list_comments(comments: &impl CommentRepository, topic_id: TopicId) -> Vec<Comment> {
    comments.list_by_topic(topic_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeTopicRepo};
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
        let comment = post_comment(
            &topics,
            &comments,
            CommentId::new(uuid::Uuid::nil()),
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
    async fn rejects_posting_to_an_unknown_topic() {
        let topics = FakeTopicRepo::new();
        let comments = FakeCommentRepo::new();
        let result = post_comment(
            &topics,
            &comments,
            CommentId::new(uuid::Uuid::nil()),
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
        let root = post_comment(
            &topics,
            &comments,
            CommentId::new(uuid::Uuid::nil()),
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
            CommentId::new(uuid::Uuid::max()),
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
        let result = post_comment(
            &topics,
            &comments,
            CommentId::new(uuid::Uuid::nil()),
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
            CommentId::new(uuid::Uuid::nil()),
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
