use crate::{CommentId, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactionKind {
    Like,
    Agree,
    Disagree,
    Thanks,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReactionKindError;

impl ReactionKind {
    pub fn parse(raw: &str) -> Result<Self, ReactionKindError> {
        match raw {
            "like" => Ok(Self::Like),
            "agree" => Ok(Self::Agree),
            "disagree" => Ok(Self::Disagree),
            "thanks" => Ok(Self::Thanks),
            _ => Err(ReactionKindError),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Like => "like",
            Self::Agree => "agree",
            Self::Disagree => "disagree",
            Self::Thanks => "thanks",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Like, Self::Agree, Self::Disagree, Self::Thanks]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactionTarget {
    Topic(TopicId),
    Comment(CommentId),
}

impl ReactionTarget {
    pub fn id(&self) -> uuid::Uuid {
        match self {
            Self::Topic(id) => id.as_uuid(),
            Self::Comment(id) => id.as_uuid(),
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Topic(_) => "topic",
            Self::Comment(_) => "comment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reaction {
    user_id: UserId,
    target: ReactionTarget,
    kind: ReactionKind,
    created_at: OffsetDateTime,
}

impl Reaction {
    pub fn new(
        user_id: UserId,
        target: ReactionTarget,
        kind: ReactionKind,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            user_id,
            target,
            kind,
            created_at,
        }
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn target(&self) -> ReactionTarget {
        self.target
    }

    pub fn kind(&self) -> ReactionKind {
        self.kind
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_known_kind_round_trip() {
        for kind in ReactionKind::all() {
            assert_eq!(ReactionKind::parse(kind.as_str()), Ok(kind));
        }
    }

    #[test]
    fn rejects_an_unknown_kind() {
        assert_eq!(ReactionKind::parse("shrug"), Err(ReactionKindError));
        assert_eq!(ReactionKind::parse(""), Err(ReactionKindError));
        assert_eq!(ReactionKind::parse("LIKE"), Err(ReactionKindError));
    }

    #[test]
    fn a_target_exposes_its_id_and_kind() {
        let topic = ReactionTarget::Topic(TopicId::new(uuid::Uuid::nil()));
        let comment = ReactionTarget::Comment(CommentId::new(uuid::Uuid::max()));
        assert_eq!(topic.kind_str(), "topic");
        assert_eq!(topic.id(), uuid::Uuid::nil());
        assert_eq!(comment.kind_str(), "comment");
        assert_eq!(comment.id(), uuid::Uuid::max());
    }

    #[test]
    fn a_reaction_exposes_its_fields() {
        let user_id = UserId::new(uuid::Uuid::nil());
        let target = ReactionTarget::Topic(TopicId::new(uuid::Uuid::max()));
        let reaction = Reaction::new(
            user_id,
            target,
            ReactionKind::Like,
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(reaction.user_id(), user_id);
        assert_eq!(reaction.target(), target);
        assert_eq!(reaction.kind(), ReactionKind::Like);
        assert_eq!(reaction.created_at(), OffsetDateTime::UNIX_EPOCH);
    }
}
