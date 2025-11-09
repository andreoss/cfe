use crate::{Body, Deletion, Revision, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommentId(uuid::Uuid);

impl CommentId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    id: CommentId,
    topic_id: TopicId,
    author_id: UserId,
    parent_id: Option<CommentId>,
    body: Body,
    created_at: OffsetDateTime,
    deleted: Option<Deletion>,
    edited: Option<Revision>,
}

impl Comment {
    pub fn new(
        id: CommentId,
        topic_id: TopicId,
        author_id: UserId,
        parent_id: Option<CommentId>,
        body: Body,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            topic_id,
            author_id,
            parent_id,
            body,
            created_at,
            deleted: None,
            edited: None,
        }
    }

    pub fn from_parts(
        id: CommentId,
        topic_id: TopicId,
        author_id: UserId,
        parent_id: Option<CommentId>,
        body: Body,
        created_at: OffsetDateTime,
        deleted: Option<Deletion>,
        edited: Option<Revision>,
    ) -> Self {
        Self {
            id,
            topic_id,
            author_id,
            parent_id,
            body,
            created_at,
            deleted,
            edited,
        }
    }

    pub fn id(&self) -> CommentId {
        self.id
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn author_id(&self) -> UserId {
        self.author_id
    }

    pub fn parent_id(&self) -> Option<CommentId> {
        self.parent_id
    }

    pub fn body(&self) -> &Body {
        &self.body
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

    pub fn revision(&self) -> Option<&Revision> {
        self.edited.as_ref()
    }

    pub fn is_edited(&self) -> bool {
        self.edited.is_some()
    }

    pub fn with_edit(&self, body: Body, revision: Revision) -> Self {
        Self {
            body,
            edited: Some(revision),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Reason;

    #[test]
    fn constructs_a_top_level_comment_not_deleted() {
        let id = CommentId::new(uuid::Uuid::nil());
        let topic_id = TopicId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let body = Body::parse("Nice topic!").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let comment = Comment::new(id, topic_id, author_id, None, body.clone(), now);
        assert_eq!(comment.id(), id);
        assert_eq!(comment.topic_id(), topic_id);
        assert_eq!(comment.author_id(), author_id);
        assert_eq!(comment.parent_id(), None);
        assert_eq!(comment.body(), &body);
        assert_eq!(comment.created_at(), now);
        assert!(!comment.is_deleted());
    }

    #[test]
    fn constructs_a_reply_with_a_parent() {
        let parent = CommentId::new(uuid::Uuid::nil());
        let id = CommentId::new(uuid::Uuid::max());
        let topic_id = TopicId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let body = Body::parse("I agree.").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let comment = Comment::new(id, topic_id, author_id, Some(parent), body, now);
        assert_eq!(comment.parent_id(), Some(parent));
    }

    #[test]
    fn with_edit_replaces_the_body_and_records_the_revision() {
        let id = CommentId::new(uuid::Uuid::nil());
        let comment = Comment::new(
            id,
            TopicId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Old body").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(!comment.is_edited());
        assert_eq!(comment.revision(), None);
        let body = Body::parse("New body").unwrap();
        let revision = Revision::new(
            UserId::new(uuid::Uuid::max()),
            OffsetDateTime::UNIX_EPOCH,
        );
        let edited = comment.with_edit(body.clone(), revision.clone());
        assert_eq!(edited.id(), comment.id());
        assert_eq!(edited.created_at(), comment.created_at());
        assert_eq!(edited.body(), &body);
        assert!(edited.is_edited());
        assert_eq!(edited.revision(), Some(&revision));
    }

    #[test]
    fn with_deletion_marks_deleted_and_keeps_identity() {
        let id = CommentId::new(uuid::Uuid::nil());
        let comment = Comment::new(
            id,
            TopicId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Nice topic!").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let deletion = Deletion::new(
            UserId::new(uuid::Uuid::max()),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let deleted = comment.with_deletion(deletion.clone());
        assert_eq!(deleted.id(), comment.id());
        assert!(deleted.is_deleted());
        assert_eq!(deleted.deletion(), Some(&deletion));
    }
}
