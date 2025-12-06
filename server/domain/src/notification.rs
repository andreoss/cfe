use crate::{CommentId, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotificationId(uuid::Uuid);

impl NotificationId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    Reply,
    Watch,
    Mention,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NotificationKindError;

impl NotificationKind {
    pub fn parse(raw: &str) -> Result<Self, NotificationKindError> {
        match raw {
            "reply" => Ok(Self::Reply),
            "watch" => Ok(Self::Watch),
            "mention" => Ok(Self::Mention),
            _ => Err(NotificationKindError),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reply => "reply",
            Self::Watch => "watch",
            Self::Mention => "mention",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Reply, Self::Watch, Self::Mention]
    }
}

impl Default for NotificationKind {
    fn default() -> Self {
        Self::Reply
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    id: NotificationId,
    recipient_id: UserId,
    actor_id: UserId,
    topic_id: TopicId,
    comment_id: CommentId,
    created_at: OffsetDateTime,
    read_at: Option<OffsetDateTime>,
    kind: NotificationKind,
}

impl Notification {
    pub fn new(
        id: NotificationId,
        recipient_id: UserId,
        actor_id: UserId,
        topic_id: TopicId,
        comment_id: CommentId,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            recipient_id,
            actor_id,
            topic_id,
            comment_id,
            created_at,
            read_at: None,
            kind: NotificationKind::Reply,
        }
    }

    pub fn of_kind(&self, kind: NotificationKind) -> Self {
        Self {
            kind,
            ..self.clone()
        }
    }

    pub fn kind(&self) -> NotificationKind {
        self.kind
    }

    pub fn from_parts(
        id: NotificationId,
        recipient_id: UserId,
        actor_id: UserId,
        topic_id: TopicId,
        comment_id: CommentId,
        created_at: OffsetDateTime,
        read_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            recipient_id,
            actor_id,
            topic_id,
            comment_id,
            created_at,
            read_at,
            kind: NotificationKind::Reply,
        }
    }

    pub fn id(&self) -> NotificationId {
        self.id
    }

    pub fn recipient_id(&self) -> UserId {
        self.recipient_id
    }

    pub fn actor_id(&self) -> UserId {
        self.actor_id
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn comment_id(&self) -> CommentId {
        self.comment_id
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn read_at(&self) -> Option<OffsetDateTime> {
        self.read_at
    }

    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }

    pub fn marked_read(&self, at: OffsetDateTime) -> Self {
        Self {
            read_at: Some(at),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification() -> Notification {
        Notification::new(
            NotificationId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::max()),
            TopicId::new(uuid::Uuid::nil()),
            CommentId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn constructs_unread_with_given_fields() {
        let n = notification();
        assert_eq!(n.recipient_id(), UserId::new(uuid::Uuid::nil()));
        assert_eq!(n.actor_id(), UserId::new(uuid::Uuid::max()));
        assert_eq!(n.created_at(), OffsetDateTime::UNIX_EPOCH);
        assert!(!n.is_read());
        assert_eq!(n.read_at(), None);
    }

    #[test]
    fn marked_read_sets_the_timestamp_and_keeps_identity() {
        let n = notification();
        let at = OffsetDateTime::UNIX_EPOCH;
        let read = n.marked_read(at);
        assert_eq!(read.id(), n.id());
        assert_eq!(read.comment_id(), n.comment_id());
        assert_eq!(read.topic_id(), n.topic_id());
        assert!(read.is_read());
        assert_eq!(read.read_at(), Some(at));
    }
}

#[cfg(test)]
mod kind_tests {
    use super::*;

    #[test]
    fn parses_every_kind_round_trip() {
        for kind in NotificationKind::all() {
            assert_eq!(NotificationKind::parse(kind.as_str()), Ok(kind));
        }
    }

    #[test]
    fn rejects_an_unknown_kind() {
        assert_eq!(
            NotificationKind::parse("something"),
            Err(NotificationKindError)
        );
        assert_eq!(NotificationKind::parse(""), Err(NotificationKindError));
    }

    #[test]
    fn a_notification_is_a_reply_unless_told_otherwise() {
        let notification = Notification::new(
            NotificationId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::max()),
            TopicId::new(uuid::Uuid::nil()),
            CommentId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(notification.kind(), NotificationKind::Reply);
        assert_eq!(NotificationKind::default(), NotificationKind::Reply);
        let watched = notification.of_kind(NotificationKind::Watch);
        assert_eq!(watched.kind(), NotificationKind::Watch);
        assert_eq!(watched.id(), notification.id());
        assert_eq!(watched.recipient_id(), notification.recipient_id());
    }
}
