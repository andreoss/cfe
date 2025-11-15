use crate::{TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    user_id: UserId,
    topic_id: TopicId,
    created_at: OffsetDateTime,
}

impl Bookmark {
    pub fn new(user_id: UserId, topic_id: TopicId, created_at: OffsetDateTime) -> Self {
        Self {
            user_id,
            topic_id,
            created_at,
        }
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_its_fields() {
        let user_id = UserId::new(uuid::Uuid::nil());
        let topic_id = TopicId::new(uuid::Uuid::max());
        let now = OffsetDateTime::UNIX_EPOCH;
        let bookmark = Bookmark::new(user_id, topic_id, now);
        assert_eq!(bookmark.user_id(), user_id);
        assert_eq!(bookmark.topic_id(), topic_id);
        assert_eq!(bookmark.created_at(), now);
    }
}
