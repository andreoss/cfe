use crate::{Address, ClientString, CommentId, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostRef {
    Topic(TopicId),
    Comment(CommentId),
}

impl PostRef {
    pub fn topic_id(&self) -> Option<TopicId> {
        match self {
            Self::Topic(id) => Some(*id),
            Self::Comment(_) => None,
        }
    }

    pub fn comment_id(&self) -> Option<CommentId> {
        match self {
            Self::Topic(_) => None,
            Self::Comment(id) => Some(*id),
        }
    }

    pub fn from_parts(topic_id: Option<TopicId>, comment_id: Option<CommentId>) -> Option<Self> {
        match (topic_id, comment_id) {
            (_, Some(id)) => Some(Self::Comment(id)),
            (Some(id), None) => Some(Self::Topic(id)),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressPost {
    user_id: UserId,
    addr: Address,
    client: Option<ClientString>,
    at: OffsetDateTime,
}

impl AddressPost {
    pub fn new(
        user_id: UserId,
        addr: Address,
        client: Option<ClientString>,
        at: OffsetDateTime,
    ) -> Self {
        Self {
            user_id,
            addr,
            client,
            at,
        }
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn addr(&self) -> &Address {
        &self.addr
    }

    pub fn client(&self) -> Option<&ClientString> {
        self.client.as_ref()
    }

    pub fn at(&self) -> OffsetDateTime {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_who_posted_from_where_and_when() {
        let post = AddressPost::new(
            UserId::new(uuid::Uuid::nil()),
            Address::parse("203.0.113.10").unwrap(),
            Some(ClientString::parse("agent/1.0").unwrap()),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(post.user_id(), UserId::new(uuid::Uuid::nil()));
        assert_eq!(post.addr().as_str(), "203.0.113.10");
        assert_eq!(post.client().map(|c| c.as_str()), Some("agent/1.0"));
        assert_eq!(post.at(), OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn a_reference_points_at_a_topic_or_a_comment() {
        let topic = PostRef::Topic(TopicId::new(uuid::Uuid::nil()));
        let comment = PostRef::Comment(CommentId::new(uuid::Uuid::max()));
        assert_eq!(topic.topic_id(), Some(TopicId::new(uuid::Uuid::nil())));
        assert_eq!(topic.comment_id(), None);
        assert_eq!(
            comment.comment_id(),
            Some(CommentId::new(uuid::Uuid::max()))
        );
        assert_eq!(comment.topic_id(), None);
    }

    #[test]
    fn a_reference_reads_back_from_stored_columns() {
        let topic_id = TopicId::new(uuid::Uuid::nil());
        let comment_id = CommentId::new(uuid::Uuid::max());
        assert_eq!(
            PostRef::from_parts(Some(topic_id), None),
            Some(PostRef::Topic(topic_id))
        );
        assert_eq!(
            PostRef::from_parts(Some(topic_id), Some(comment_id)),
            Some(PostRef::Comment(comment_id))
        );
        assert_eq!(PostRef::from_parts(None, None), None);
    }

    #[test]
    fn a_client_string_is_optional() {
        let post = AddressPost::new(
            UserId::new(uuid::Uuid::nil()),
            Address::parse("203.0.113.10").unwrap(),
            None,
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(post.client(), None);
    }
}
