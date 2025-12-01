use crate::ports::{CommentRepository, TopicRepository, VersionRepository};
use domain::{
    Body, Comment, CommentId, Revision, TagSet, Title, Topic, TopicId, User, Version, VersionId,
    VersionOf,
};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum EditError {
    NotFound,
    NotAuthorized,
    Deleted,
}

fn may_edit(user: &User, author_id: domain::UserId) -> bool {
    user.id() == author_id || user.role().may_correct()
}

#[allow(clippy::too_many_arguments)]
pub async fn edit_topic(
    topics: &(impl TopicRepository + ?Sized),
    versions: &(impl VersionRepository + ?Sized),
    user: &User,
    topic_id: TopicId,
    title: Title,
    body: Body,
    tags: TagSet,
    version_id: VersionId,
    now: OffsetDateTime,
) -> Result<Topic, EditError> {
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(EditError::NotFound)?;
    if !may_edit(user, topic.author_id()) {
        return Err(EditError::NotAuthorized);
    }
    if topic.is_deleted() {
        return Err(EditError::Deleted);
    }
    versions
        .save(&Version::new(
            version_id,
            VersionOf::Topic,
            topic.id().as_uuid(),
            Some(topic.title().clone()),
            topic.body().clone(),
            user.id(),
            now,
        ))
        .await;
    let edited = topic.with_edit(title, body, tags, Revision::new(user.id(), now));
    topics.update(&edited).await;
    Ok(edited)
}

pub async fn edit_comment(
    comments: &(impl CommentRepository + ?Sized),
    versions: &(impl VersionRepository + ?Sized),
    user: &User,
    comment_id: CommentId,
    body: Body,
    version_id: VersionId,
    now: OffsetDateTime,
) -> Result<Comment, EditError> {
    let comment = comments
        .find_by_id(comment_id)
        .await
        .ok_or(EditError::NotFound)?;
    if !may_edit(user, comment.author_id()) {
        return Err(EditError::NotAuthorized);
    }
    if comment.is_deleted() {
        return Err(EditError::Deleted);
    }
    versions
        .save(&Version::new(
            version_id,
            VersionOf::Comment,
            comment.id().as_uuid(),
            None,
            comment.body().clone(),
            user.id(),
            now,
        ))
        .await;
    let edited = comment.with_edit(body, Revision::new(user.id(), now));
    comments.update(&edited).await;
    Ok(edited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeTopicRepo, FakeVersionRepo};
    use domain::{Deletion, Email, Reason, SectionId, UserId, Username};

    fn author_id() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn author() -> User {
        User::register(
            author_id(),
            Username::parse("author_01").unwrap(),
            Email::parse("author@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn stranger() -> User {
        User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("stranger_01").unwrap(),
            Email::parse("stranger@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn moderator() -> User {
        User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("mod_01").unwrap(),
            Email::parse("mod@example.com").unwrap(),
            "hash".to_owned(),
        )
        .promoted_to_moderator()
    }

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            author_id(),
            Title::parse("Before").unwrap(),
            Body::parse("Old body").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn comment() -> Comment {
        Comment::new(
            CommentId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::nil()),
            author_id(),
            None,
            Body::parse("Old body").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn new_title() -> Title {
        Title::parse("After").unwrap()
    }

    fn new_body() -> Body {
        Body::parse("New body").unwrap()
    }

    #[tokio::test]
    async fn editing_keeps_what_was_there_before() {
        let topics = FakeTopicRepo::with(topic());
        let versions = FakeVersionRepo::new();
        edit_topic(
            &topics,
            &versions,
            &author(),
            topic().id(),
            new_title(),
            new_body(),
            TagSet::empty(),
            VersionId::new(uuid::Uuid::from_u128(5)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let kept = versions
            .list_for(domain::VersionOf::Topic, topic().id().as_uuid())
            .await;
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].body().as_str(), "Old body");
        assert_eq!(kept[0].title().map(|t| t.as_str()), Some("Before"));
    }

    #[tokio::test]
    async fn a_refused_edit_keeps_no_version() {
        let topics = FakeTopicRepo::with(topic());
        let versions = FakeVersionRepo::new();
        let stranger = User::register(
            UserId::new(uuid::Uuid::from_u128(77)),
            Username::parse("stranger_01").unwrap(),
            Email::parse("stranger@example.com").unwrap(),
            "hash".to_owned(),
        );
        assert!(
            edit_topic(
                &topics,
                &versions,
                &stranger,
                topic().id(),
                new_title(),
                new_body(),
                TagSet::empty(),
                VersionId::new(uuid::Uuid::from_u128(5)),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .is_err()
        );
        assert!(
            versions
                .list_for(domain::VersionOf::Topic, topic().id().as_uuid())
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn author_edits_own_topic() {
        let topics = FakeTopicRepo::with(topic());
        let edited = edit_topic(
            &topics,
            &FakeVersionRepo::new(),
            &author(),
            topic().id(),
            new_title(),
            new_body(),
            TagSet::empty(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(edited.title(), &new_title());
        assert_eq!(edited.body(), &new_body());
        assert!(edited.is_edited());
        assert_eq!(edited.revision().unwrap().editor_id(), author_id());
    }

    #[tokio::test]
    async fn moderator_edits_another_authors_topic() {
        let topics = FakeTopicRepo::with(topic());
        let edited = edit_topic(
            &topics,
            &FakeVersionRepo::new(),
            &moderator(),
            topic().id(),
            new_title(),
            new_body(),
            TagSet::empty(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(edited.is_edited());
    }

    #[tokio::test]
    async fn stranger_cannot_edit_someone_elses_topic() {
        let topics = FakeTopicRepo::with(topic());
        let result = edit_topic(
            &topics,
            &FakeVersionRepo::new(),
            &stranger(),
            topic().id(),
            new_title(),
            new_body(),
            TagSet::empty(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EditError::NotAuthorized));
    }

    #[tokio::test]
    async fn rejects_editing_an_unknown_topic() {
        let topics = FakeTopicRepo::new();
        let result = edit_topic(
            &topics,
            &FakeVersionRepo::new(),
            &author(),
            topic().id(),
            new_title(),
            new_body(),
            TagSet::empty(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EditError::NotFound));
    }

    #[tokio::test]
    async fn rejects_editing_a_deleted_topic() {
        let deletion = Deletion::new(
            UserId::new(uuid::Uuid::max()),
            Reason::parse("spam").unwrap(),
            domain::Penalty::default(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let topics = FakeTopicRepo::with(topic().with_deletion(deletion));
        let result = edit_topic(
            &topics,
            &FakeVersionRepo::new(),
            &author(),
            topic().id(),
            new_title(),
            new_body(),
            TagSet::empty(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EditError::Deleted));
    }

    #[tokio::test]
    async fn author_edits_own_comment() {
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        let edited = edit_comment(
            &comments,
            &FakeVersionRepo::new(),
            &author(),
            comment().id(),
            new_body(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(edited.body(), &new_body());
        assert!(edited.is_edited());
    }

    #[tokio::test]
    async fn stranger_cannot_edit_someone_elses_comment() {
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        let result = edit_comment(
            &comments,
            &FakeVersionRepo::new(),
            &stranger(),
            comment().id(),
            new_body(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EditError::NotAuthorized));
    }

    #[tokio::test]
    async fn rejects_editing_a_deleted_comment() {
        let deletion = Deletion::new(
            UserId::new(uuid::Uuid::max()),
            Reason::parse("spam").unwrap(),
            domain::Penalty::default(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let comments = FakeCommentRepo::new();
        comments.save(&comment().with_deletion(deletion)).await;
        let result = edit_comment(
            &comments,
            &FakeVersionRepo::new(),
            &author(),
            comment().id(),
            new_body(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EditError::Deleted));
    }

    #[tokio::test]
    async fn rejects_editing_an_unknown_comment() {
        let comments = FakeCommentRepo::new();
        let result = edit_comment(
            &comments,
            &FakeVersionRepo::new(),
            &author(),
            comment().id(),
            new_body(),
            VersionId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(EditError::NotFound));
    }
}
