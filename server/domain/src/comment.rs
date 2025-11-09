use crate::{Body, TopicId, UserId};
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_a_top_level_comment() {
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
}
