use crate::{TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    user_id: UserId,
    topic_id: TopicId,
    created_at: OffsetDateTime,
}

impl Watch {
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
    fn a_watch_names_who_watches_what_and_when() {
        let watch = Watch::new(
            UserId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::max()),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(watch.user_id(), UserId::new(uuid::Uuid::nil()));
        assert_eq!(watch.topic_id(), TopicId::new(uuid::Uuid::max()));
        assert_eq!(watch.created_at(), OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn two_watches_on_the_same_pair_are_equal() {
        let one = Watch::new(
            UserId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::max()),
            OffsetDateTime::UNIX_EPOCH,
        );
        let other = one.clone();
        assert_eq!(one, other);
    }
}
