use crate::ports::{CommentRepository, TopicRepository};
use domain::{Comment, CommentId, Deletion, Reason, Topic, TopicId, User};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum DeleteError {
    NotFound,
    NotAuthorized,
}

pub async fn delete_topic(
    topics: &impl TopicRepository,
    moderator: &User,
    topic_id: TopicId,
    reason: Reason,
    now: OffsetDateTime,
) -> Result<Topic, DeleteError> {
    if !moderator.role().is_moderator() {
        return Err(DeleteError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(DeleteError::NotFound)?;
    let deletion = Deletion::new(moderator.id(), reason, now);
    let deleted = topic.with_deletion(deletion);
    topics.update(&deleted).await;
    Ok(deleted)
}

pub async fn delete_comment(
    comments: &impl CommentRepository,
    moderator: &User,
    comment_id: CommentId,
    reason: Reason,
    now: OffsetDateTime,
) -> Result<Comment, DeleteError> {
    if !moderator.role().is_moderator() {
        return Err(DeleteError::NotAuthorized);
    }
    let comment = comments
        .find_by_id(comment_id)
        .await
        .ok_or(DeleteError::NotFound)?;
    let deletion = Deletion::new(moderator.id(), reason, now);
    let deleted = comment.with_deletion(deletion);
    comments.update(&deleted).await;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeTopicRepo};
    use domain::{Body, Email, SectionId, TagSet, Title, UserId, Username};

    fn moderator() -> User {
        User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("mod_01").unwrap(),
            Email::parse("mod@example.com").unwrap(),
            "hash".to_owned(),
        )
        .promoted_to_moderator()
    }

    fn plain_user() -> User {
        User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("plain_01").unwrap(),
            Email::parse("plain@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

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

    fn comment() -> Comment {
        Comment::new(
            CommentId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Nice topic!").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn moderator_can_delete_a_topic() {
        let topics = FakeTopicRepo::with(topic());
        let deleted = delete_topic(
            &topics,
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(deleted.is_deleted());
    }

    #[tokio::test]
    async fn plain_user_cannot_delete_a_topic() {
        let topics = FakeTopicRepo::with(topic());
        let result = delete_topic(
            &topics,
            &plain_user(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(DeleteError::NotAuthorized));
    }

    #[tokio::test]
    async fn rejects_deleting_an_unknown_topic() {
        let topics = FakeTopicRepo::new();
        let result = delete_topic(
            &topics,
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(DeleteError::NotFound));
    }

    #[tokio::test]
    async fn moderator_can_delete_a_comment() {
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        let deleted = delete_comment(
            &comments,
            &moderator(),
            comment().id(),
            Reason::parse("off-topic").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(deleted.is_deleted());
    }

    #[tokio::test]
    async fn plain_user_cannot_delete_a_comment() {
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        let result = delete_comment(
            &comments,
            &plain_user(),
            comment().id(),
            Reason::parse("off-topic").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(DeleteError::NotAuthorized));
    }
}
