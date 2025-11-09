use crate::{Body, Deletion, SectionId, TagSet, Title, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TopicId(uuid::Uuid);

impl TopicId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    id: TopicId,
    section_id: SectionId,
    author_id: UserId,
    title: Title,
    body: Body,
    tags: TagSet,
    created_at: OffsetDateTime,
    deleted: Option<Deletion>,
}

impl Topic {
    pub fn new(
        id: TopicId,
        section_id: SectionId,
        author_id: UserId,
        title: Title,
        body: Body,
        tags: TagSet,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            section_id,
            author_id,
            title,
            body,
            tags,
            created_at,
            deleted: None,
        }
    }

    pub fn from_parts(
        id: TopicId,
        section_id: SectionId,
        author_id: UserId,
        title: Title,
        body: Body,
        tags: TagSet,
        created_at: OffsetDateTime,
        deleted: Option<Deletion>,
    ) -> Self {
        Self {
            id,
            section_id,
            author_id,
            title,
            body,
            tags,
            created_at,
            deleted,
        }
    }

    pub fn id(&self) -> TopicId {
        self.id
    }

    pub fn section_id(&self) -> SectionId {
        self.section_id
    }

    pub fn author_id(&self) -> UserId {
        self.author_id
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn tags(&self) -> &TagSet {
        &self.tags
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn deletion(&self) -> Option<&Deletion> {
        self.deleted.as_ref()
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted.is_some()
    }

    pub fn with_deletion(&self, deletion: Deletion) -> Self {
        Self {
            deleted: Some(deletion),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Reason;

    #[test]
    fn constructs_with_given_fields_and_not_deleted() {
        let id = TopicId::new(uuid::Uuid::nil());
        let section_id = SectionId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let title = Title::parse("Hello").unwrap();
        let body = Body::parse("World").unwrap();
        let tags = TagSet::parse(&["rust".to_string()]).unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let topic = Topic::new(
            id,
            section_id,
            author_id,
            title.clone(),
            body.clone(),
            tags.clone(),
            now,
        );
        assert_eq!(topic.id(), id);
        assert_eq!(topic.section_id(), section_id);
        assert_eq!(topic.author_id(), author_id);
        assert_eq!(topic.title(), &title);
        assert_eq!(topic.body(), &body);
        assert_eq!(topic.tags(), &tags);
        assert_eq!(topic.created_at(), now);
        assert!(!topic.is_deleted());
        assert_eq!(topic.deletion(), None);
    }

    #[test]
    fn with_deletion_marks_deleted_and_keeps_identity() {
        let id = TopicId::new(uuid::Uuid::nil());
        let section_id = SectionId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let topic = Topic::new(
            id,
            section_id,
            author_id,
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let deletion = Deletion::new(
            UserId::new(uuid::Uuid::max()),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let deleted = topic.with_deletion(deletion.clone());
        assert_eq!(deleted.id(), topic.id());
        assert!(deleted.is_deleted());
        assert_eq!(deleted.deletion(), Some(&deletion));
    }
}
