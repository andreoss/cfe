use crate::ports::{CommentRepository, TopicRepository, UserRepository};
use crate::reputation;
use domain::{Comment, CommentId, Deletion, Penalty, Reason, Topic, TopicId, User};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum DeleteError {
    NotFound,
    NotAuthorized,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RestoreError {
    NotFound,
    NotAuthorized,
    NotDeleted,
}

pub async fn delete_topic(
    topics: &(impl TopicRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
    reason: Reason,
    penalty: Penalty,
    now: OffsetDateTime,
) -> Result<Topic, DeleteError> {
    if !moderator.role().is_moderator() {
        return Err(DeleteError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(DeleteError::NotFound)?;
    let deletion = Deletion::new(moderator.id(), reason, penalty, now);
    let deleted = topic.with_deletion(deletion);
    topics.update(&deleted).await;
    reputation::apply_deletion(users, deleted.author_id(), penalty).await;
    Ok(deleted)
}

pub async fn delete_comment(
    comments: &(impl CommentRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    moderator: &User,
    comment_id: CommentId,
    reason: Reason,
    penalty: Penalty,
    now: OffsetDateTime,
) -> Result<Comment, DeleteError> {
    if !moderator.role().is_moderator() {
        return Err(DeleteError::NotAuthorized);
    }
    let comment = comments
        .find_by_id(comment_id)
        .await
        .ok_or(DeleteError::NotFound)?;
    let deletion = Deletion::new(moderator.id(), reason, penalty, now);
    let deleted = comment.with_deletion(deletion);
    comments.update(&deleted).await;
    reputation::apply_deletion(users, deleted.author_id(), penalty).await;
    Ok(deleted)
}

pub async fn restore_topic(
    topics: &(impl TopicRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
) -> Result<Topic, RestoreError> {
    if !moderator.role().is_moderator() {
        return Err(RestoreError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(RestoreError::NotFound)?;
    let deletion = topic.deletion().ok_or(RestoreError::NotDeleted)?.clone();
    let restored = topic.restored();
    topics.update(&restored).await;
    reputation::give_back(users, restored.author_id(), deletion.penalty()).await;
    Ok(restored)
}

pub async fn restore_comment(
    comments: &(impl CommentRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    moderator: &User,
    comment_id: CommentId,
) -> Result<Comment, RestoreError> {
    if !moderator.role().is_moderator() {
        return Err(RestoreError::NotAuthorized);
    }
    let comment = comments
        .find_by_id(comment_id)
        .await
        .ok_or(RestoreError::NotFound)?;
    let deletion = comment.deletion().ok_or(RestoreError::NotDeleted)?.clone();
    let restored = comment.restored();
    comments.update(&restored).await;
    reputation::give_back(users, restored.author_id(), deletion.penalty()).await;
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeTopicRepo, FakeUserRepo};
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
            &FakeUserRepo::new(),
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::default(),
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
            &FakeUserRepo::new(),
            &plain_user(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::default(),
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
            &FakeUserRepo::new(),
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::default(),
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
            &FakeUserRepo::new(),
            &moderator(),
            comment().id(),
            Reason::parse("off-topic").unwrap(),
            Penalty::default(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(deleted.is_deleted());
    }

    #[tokio::test]
    async fn deleting_a_topic_costs_its_author() {
        let topics = FakeTopicRepo::with(topic());
        let users = FakeUserRepo::new();
        let author = User::register(
            topic().author_id(),
            Username::parse("author_01").unwrap(),
            Email::parse("author@example.com").unwrap(),
            "hash".to_owned(),
        );
        users.save(&author).await;
        delete_topic(
            &topics,
            &users,
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::default(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let after = users.find_by_id(author.id()).await.unwrap();
        assert_eq!(after.score().value(), domain::for_deletion());
    }

    #[tokio::test]
    async fn a_refused_deletion_leaves_the_score_alone() {
        let topics = FakeTopicRepo::with(topic());
        let users = FakeUserRepo::new();
        let author = User::register(
            topic().author_id(),
            Username::parse("author_02").unwrap(),
            Email::parse("author2@example.com").unwrap(),
            "hash".to_owned(),
        );
        users.save(&author).await;
        let result = delete_topic(
            &topics,
            &users,
            &plain_user(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::default(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(DeleteError::NotAuthorized));
        let after = users.find_by_id(author.id()).await.unwrap();
        assert_eq!(after.score().value(), 0);
    }

    #[tokio::test]
    async fn the_penalty_the_moderator_chose_is_what_the_author_pays() {
        let topics = FakeTopicRepo::with(topic());
        let users = FakeUserRepo::new();
        let author = User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("author_01").unwrap(),
            Email::parse("author@example.com").unwrap(),
            "hash".to_owned(),
        );
        users.save(&author).await;
        delete_topic(
            &topics,
            &users,
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::parse(-30).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let stored = users.find_by_id(author.id()).await.unwrap();
        assert_eq!(stored.score().value(), -30);
    }

    #[tokio::test]
    async fn a_penalty_of_nothing_costs_the_author_nothing() {
        let topics = FakeTopicRepo::with(topic());
        let users = FakeUserRepo::new();
        let author = User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("author_01").unwrap(),
            Email::parse("author@example.com").unwrap(),
            "hash".to_owned(),
        );
        users.save(&author).await;
        delete_topic(
            &topics,
            &users,
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::parse(0).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let stored = users.find_by_id(author.id()).await.unwrap();
        assert_eq!(stored.score().value(), 0);
    }

    #[tokio::test]
    async fn restoring_gives_back_exactly_what_was_taken() {
        let topics = FakeTopicRepo::with(topic());
        let users = FakeUserRepo::new();
        let author = User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("author_01").unwrap(),
            Email::parse("author@example.com").unwrap(),
            "hash".to_owned(),
        );
        users.save(&author).await;
        delete_topic(
            &topics,
            &users,
            &moderator(),
            topic().id(),
            Reason::parse("spam").unwrap(),
            Penalty::parse(-30).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(
            users.find_by_id(author.id()).await.unwrap().score().value(),
            -30
        );
        let restored = restore_topic(&topics, &users, &moderator(), topic().id())
            .await
            .unwrap();
        assert!(!restored.is_deleted());
        assert_eq!(
            users.find_by_id(author.id()).await.unwrap().score().value(),
            0
        );
    }

    #[tokio::test]
    async fn restoring_something_that_was_never_deleted_is_refused() {
        let topics = FakeTopicRepo::with(topic());
        assert_eq!(
            restore_topic(&topics, &FakeUserRepo::new(), &moderator(), topic().id()).await,
            Err(RestoreError::NotDeleted)
        );
    }

    #[tokio::test]
    async fn only_a_moderator_may_restore() {
        let topics = FakeTopicRepo::with(topic());
        assert_eq!(
            restore_topic(&topics, &FakeUserRepo::new(), &plain_user(), topic().id()).await,
            Err(RestoreError::NotAuthorized)
        );
    }

    #[tokio::test]
    async fn restoring_something_that_is_not_there_is_refused() {
        let topics = FakeTopicRepo::new();
        assert_eq!(
            restore_topic(&topics, &FakeUserRepo::new(), &moderator(), topic().id()).await,
            Err(RestoreError::NotFound)
        );
    }

    #[tokio::test]
    async fn a_corrector_cannot_delete_a_topic_or_a_comment() {
        let corrector = plain_user().with_role(domain::Role::Corrector);
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        assert_eq!(
            delete_topic(
                &topics,
                &FakeUserRepo::new(),
                &corrector,
                topic().id(),
                Reason::parse("spam").unwrap(),
                Penalty::default(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await,
            Err(DeleteError::NotAuthorized)
        );
        assert_eq!(
            delete_comment(
                &comments,
                &FakeUserRepo::new(),
                &corrector,
                comment().id(),
                Reason::parse("spam").unwrap(),
                Penalty::default(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await,
            Err(DeleteError::NotAuthorized)
        );
    }

    #[tokio::test]
    async fn plain_user_cannot_delete_a_comment() {
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        let result = delete_comment(
            &comments,
            &FakeUserRepo::new(),
            &plain_user(),
            comment().id(),
            Reason::parse("off-topic").unwrap(),
            Penalty::default(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(DeleteError::NotAuthorized));
    }
}
