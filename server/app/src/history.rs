use crate::ports::{CommentRepository, TopicRepository, VersionRepository};
use domain::{Change, CommentId, TopicId, Version, VersionId, VersionOf, difference};

#[derive(Debug, PartialEq, Eq)]
pub enum HistoryError {
    NotFound,
}

pub async fn topic_history(
    versions: &(impl VersionRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    topic_id: TopicId,
) -> Result<Vec<Version>, HistoryError> {
    topics
        .find_by_id(topic_id)
        .await
        .ok_or(HistoryError::NotFound)?;
    Ok(versions
        .list_for(VersionOf::Topic, topic_id.as_uuid())
        .await)
}

pub async fn comment_history(
    versions: &(impl VersionRepository + ?Sized),
    comments: &(impl CommentRepository + ?Sized),
    comment_id: CommentId,
) -> Result<Vec<Version>, HistoryError> {
    comments
        .find_by_id(comment_id)
        .await
        .ok_or(HistoryError::NotFound)?;
    Ok(versions
        .list_for(VersionOf::Comment, comment_id.as_uuid())
        .await)
}

pub async fn what_changed(
    versions: &(impl VersionRepository + ?Sized),
    from: VersionId,
    to: &str,
) -> Result<Vec<Change>, HistoryError> {
    let earlier = versions.find(from).await.ok_or(HistoryError::NotFound)?;
    Ok(difference(earlier.body().as_str(), to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeTopicRepo, FakeVersionRepo};
    use domain::{Body, Comment, SectionId, TagSet, Title, Topic, UserId};
    use time::OffsetDateTime;

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::from_u128(1)),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::from_u128(9)),
            Title::parse("A subject").unwrap(),
            Body::parse("first line\nsecond line").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn comment() -> Comment {
        Comment::new(
            CommentId::new(uuid::Uuid::from_u128(2)),
            TopicId::new(uuid::Uuid::from_u128(1)),
            UserId::new(uuid::Uuid::from_u128(9)),
            None,
            Body::parse("a remark").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn version(n: u128, of: VersionOf, subject: uuid::Uuid, body: &str) -> Version {
        Version::new(
            VersionId::new(uuid::Uuid::from_u128(n)),
            of,
            subject,
            None,
            Body::parse(body).unwrap(),
            UserId::new(uuid::Uuid::from_u128(9)),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn a_subject_with_no_edits_has_no_history() {
        let versions = FakeVersionRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let found = topic_history(&versions, &topics, topic().id())
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn every_version_kept_for_a_subject_is_listed() {
        let versions = FakeVersionRepo::new();
        let topics = FakeTopicRepo::with(topic());
        versions
            .save(&version(1, VersionOf::Topic, topic().id().as_uuid(), "one"))
            .await;
        versions
            .save(&version(2, VersionOf::Topic, topic().id().as_uuid(), "two"))
            .await;
        let found = topic_history(&versions, &topics, topic().id())
            .await
            .unwrap();
        assert_eq!(found.len(), 2);
    }

    #[tokio::test]
    async fn a_comment_keeps_its_own_history_apart_from_a_subject() {
        let versions = FakeVersionRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        versions
            .save(&version(1, VersionOf::Topic, topic().id().as_uuid(), "one"))
            .await;
        versions
            .save(&version(
                2,
                VersionOf::Comment,
                comment().id().as_uuid(),
                "two",
            ))
            .await;
        assert_eq!(
            topic_history(&versions, &topics, topic().id())
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            comment_history(&versions, &comments, comment().id())
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn asking_about_something_that_is_not_there_is_refused() {
        let versions = FakeVersionRepo::new();
        let topics = FakeTopicRepo::new();
        assert_eq!(
            topic_history(&versions, &topics, topic().id()).await,
            Err(HistoryError::NotFound)
        );
    }

    #[tokio::test]
    async fn the_difference_is_taken_against_what_is_there_now() {
        let versions = FakeVersionRepo::new();
        versions
            .save(&version(
                1,
                VersionOf::Topic,
                topic().id().as_uuid(),
                "first line\nsecond line",
            ))
            .await;
        let changes = what_changed(
            &versions,
            VersionId::new(uuid::Uuid::from_u128(1)),
            "first line\nsecond line changed",
        )
        .await
        .unwrap();
        assert_eq!(
            changes,
            vec![
                Change::Kept("first line".to_owned()),
                Change::Removed("second line".to_owned()),
                Change::Added("second line changed".to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn a_version_nobody_kept_is_refused() {
        let versions = FakeVersionRepo::new();
        assert_eq!(
            what_changed(&versions, VersionId::new(uuid::Uuid::from_u128(404)), "x").await,
            Err(HistoryError::NotFound)
        );
    }
}
