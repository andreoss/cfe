use crate::{Body, SectionId, Title, UserId};
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
    created_at: OffsetDateTime,
}

impl Topic {
    pub fn new(
        id: TopicId,
        section_id: SectionId,
        author_id: UserId,
        title: Title,
        body: Body,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            section_id,
            author_id,
            title,
            body,
            created_at,
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

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_with_given_fields() {
        let id = TopicId::new(uuid::Uuid::nil());
        let section_id = SectionId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let title = Title::parse("Hello").unwrap();
        let body = Body::parse("World").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let topic = Topic::new(id, section_id, author_id, title.clone(), body.clone(), now);
        assert_eq!(topic.id(), id);
        assert_eq!(topic.section_id(), section_id);
        assert_eq!(topic.author_id(), author_id);
        assert_eq!(topic.title(), &title);
        assert_eq!(topic.body(), &body);
        assert_eq!(topic.created_at(), now);
    }
}
