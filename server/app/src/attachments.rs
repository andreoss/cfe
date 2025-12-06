use crate::ports::{AttachmentRepository, TopicRepository};
use domain::{ATTACHMENT_MAX_PER_TOPIC, Attachment, AttachmentError, AttachmentId, TopicId, User};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum AttachError {
    TopicNotFound,
    NotAuthorized,
    TooMany,
    Rejected(AttachmentError),
}

pub async fn attach_image(
    attachments: &(impl AttachmentRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    uploader: &User,
    id: AttachmentId,
    topic_id: TopicId,
    bytes: Vec<u8>,
    now: OffsetDateTime,
) -> Result<Attachment, AttachError> {
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(AttachError::TopicNotFound)?;
    if topic.author_id() != uploader.id() && !uploader.role().is_moderator() {
        return Err(AttachError::NotAuthorized);
    }
    if attachments.count_for(topic_id).await as usize >= ATTACHMENT_MAX_PER_TOPIC {
        return Err(AttachError::TooMany);
    }
    let attachment = Attachment::parse(id, topic_id, bytes, uploader.id(), now)
        .map_err(AttachError::Rejected)?;
    attachments.save(&attachment).await;
    Ok(attachment)
}

pub async fn images_on(
    attachments: &(impl AttachmentRepository + ?Sized),
    topic_id: TopicId,
) -> Vec<Attachment> {
    attachments.list_for(topic_id).await
}

pub async fn image(
    attachments: &(impl AttachmentRepository + ?Sized),
    id: AttachmentId,
) -> Option<Attachment> {
    attachments.find(id).await
}

pub async fn remove_image(
    attachments: &(impl AttachmentRepository + ?Sized),
    remover: &User,
    id: AttachmentId,
) -> Result<(), AttachError> {
    let attachment = attachments
        .find(id)
        .await
        .ok_or(AttachError::TopicNotFound)?;
    if attachment.uploaded_by() != remover.id() && !remover.role().is_moderator() {
        return Err(AttachError::NotAuthorized);
    }
    attachments.delete(id).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeAttachmentRepo, FakeTopicRepo};
    use domain::{Body, Email, SectionId, TagSet, Title, Topic, UserId, Username};

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn author() -> User {
        user(9, "author_01")
    }

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::from_u128(2)),
            SectionId::new(uuid::Uuid::nil()),
            author().id(),
            Title::parse("A subject").unwrap(),
            Body::parse("A body.").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn id(n: u128) -> AttachmentId {
        AttachmentId::new(uuid::Uuid::from_u128(n))
    }

    async fn attach(
        repo: &FakeAttachmentRepo,
        topics: &FakeTopicRepo,
        who: &User,
        n: u128,
    ) -> Result<Attachment, AttachError> {
        attach_image(
            repo,
            topics,
            who,
            id(n),
            topic().id(),
            PNG.to_vec(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
    }

    #[tokio::test]
    async fn an_author_attaches_an_image_to_their_own_subject() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let attached = attach(&repo, &topics, &author(), 1).await.unwrap();
        assert_eq!(attached.topic_id(), topic().id());
        assert_eq!(images_on(&repo, topic().id()).await.len(), 1);
    }

    #[tokio::test]
    async fn a_moderator_may_attach_to_somebody_elses() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let moderator = user(1, "keeper_01").promoted_to_moderator();
        assert!(attach(&repo, &topics, &moderator, 1).await.is_ok());
    }

    #[tokio::test]
    async fn a_stranger_may_not() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        assert_eq!(
            attach(&repo, &topics, &user(5, "stranger_01"), 1).await,
            Err(AttachError::NotAuthorized)
        );
        assert!(images_on(&repo, topic().id()).await.is_empty());
    }

    #[tokio::test]
    async fn attaching_to_a_subject_that_is_not_there_is_refused() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::new();
        assert_eq!(
            attach(&repo, &topics, &author(), 1).await,
            Err(AttachError::TopicNotFound)
        );
    }

    #[tokio::test]
    async fn something_that_is_not_an_image_is_refused() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        let result = attach_image(
            &repo,
            &topics,
            &author(),
            id(1),
            topic().id(),
            b"<svg onload=alert(1)>".to_vec(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(
            result,
            Err(AttachError::Rejected(AttachmentError::UnsupportedFormat))
        );
    }

    #[tokio::test]
    async fn a_subject_may_not_carry_more_than_the_limit() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        for n in 0..ATTACHMENT_MAX_PER_TOPIC {
            assert!(
                attach(&repo, &topics, &author(), n as u128 + 1)
                    .await
                    .is_ok()
            );
        }
        assert_eq!(
            attach(&repo, &topics, &author(), 99).await,
            Err(AttachError::TooMany)
        );
    }

    #[tokio::test]
    async fn one_image_is_found_by_itself() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        attach(&repo, &topics, &author(), 1).await.unwrap();
        assert!(image(&repo, id(1)).await.is_some());
        assert!(image(&repo, id(404)).await.is_none());
    }

    #[tokio::test]
    async fn whoever_attached_it_may_take_it_away() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        attach(&repo, &topics, &author(), 1).await.unwrap();
        assert!(remove_image(&repo, &author(), id(1)).await.is_ok());
        assert!(images_on(&repo, topic().id()).await.is_empty());
    }

    #[tokio::test]
    async fn a_moderator_may_take_it_away_too_and_a_stranger_may_not() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        attach(&repo, &topics, &author(), 1).await.unwrap();
        assert_eq!(
            remove_image(&repo, &user(5, "stranger_01"), id(1)).await,
            Err(AttachError::NotAuthorized)
        );
        let moderator = user(1, "keeper_01").promoted_to_moderator();
        assert!(remove_image(&repo, &moderator, id(1)).await.is_ok());
    }

    #[tokio::test]
    async fn taking_away_room_lets_another_be_attached() {
        let repo = FakeAttachmentRepo::new();
        let topics = FakeTopicRepo::with(topic());
        for n in 0..ATTACHMENT_MAX_PER_TOPIC {
            attach(&repo, &topics, &author(), n as u128 + 1)
                .await
                .unwrap();
        }
        remove_image(&repo, &author(), id(1)).await.unwrap();
        assert!(attach(&repo, &topics, &author(), 99).await.is_ok());
    }
}
