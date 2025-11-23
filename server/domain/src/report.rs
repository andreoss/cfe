use crate::{CommentId, Reason, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReportId(uuid::Uuid);

impl ReportId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportKind {
    Rule,
    Spelling,
    Tag,
    Group,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReportKindError;

impl ReportKind {
    pub fn parse(raw: &str) -> Result<Self, ReportKindError> {
        match raw {
            "rule" => Ok(Self::Rule),
            "spelling" => Ok(Self::Spelling),
            "tag" => Ok(Self::Tag),
            "group" => Ok(Self::Group),
            _ => Err(ReportKindError),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Spelling => "spelling",
            Self::Tag => "tag",
            Self::Group => "group",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Rule, Self::Spelling, Self::Tag, Self::Group]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportTarget {
    Topic(TopicId),
    Comment(TopicId, CommentId),
}

impl ReportTarget {
    pub fn topic_id(&self) -> TopicId {
        match self {
            Self::Topic(id) => *id,
            Self::Comment(topic_id, _) => *topic_id,
        }
    }

    pub fn comment_id(&self) -> Option<CommentId> {
        match self {
            Self::Topic(_) => None,
            Self::Comment(_, id) => Some(*id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    id: ReportId,
    target: ReportTarget,
    reporter_id: UserId,
    kind: ReportKind,
    reason: Reason,
    created_at: OffsetDateTime,
    closed_by: Option<UserId>,
    closed_at: Option<OffsetDateTime>,
}

impl Report {
    pub fn open(
        id: ReportId,
        target: ReportTarget,
        reporter_id: UserId,
        kind: ReportKind,
        reason: Reason,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            target,
            reporter_id,
            kind,
            reason,
            created_at,
            closed_by: None,
            closed_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: ReportId,
        target: ReportTarget,
        reporter_id: UserId,
        kind: ReportKind,
        reason: Reason,
        created_at: OffsetDateTime,
        closed_by: Option<UserId>,
        closed_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            target,
            reporter_id,
            kind,
            reason,
            created_at,
            closed_by,
            closed_at,
        }
    }

    pub fn id(&self) -> ReportId {
        self.id
    }

    pub fn target(&self) -> ReportTarget {
        self.target
    }

    pub fn reporter_id(&self) -> UserId {
        self.reporter_id
    }

    pub fn kind(&self) -> ReportKind {
        self.kind
    }

    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn closed_by(&self) -> Option<UserId> {
        self.closed_by
    }

    pub fn closed_at(&self) -> Option<OffsetDateTime> {
        self.closed_at
    }

    pub fn is_open(&self) -> bool {
        self.closed_at.is_none()
    }

    pub fn closed(&self, by: UserId, at: OffsetDateTime) -> Self {
        Self {
            closed_by: Some(by),
            closed_at: Some(at),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason() -> Reason {
        Reason::parse("this breaks the rules").unwrap()
    }

    fn topic_target() -> ReportTarget {
        ReportTarget::Topic(TopicId::new(uuid::Uuid::nil()))
    }

    fn report() -> Report {
        Report::open(
            ReportId::new(uuid::Uuid::nil()),
            topic_target(),
            UserId::new(uuid::Uuid::max()),
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn parses_every_kind_round_trip() {
        for kind in ReportKind::all() {
            assert_eq!(ReportKind::parse(kind.as_str()), Ok(kind));
        }
    }

    #[test]
    fn rejects_an_unknown_kind() {
        assert_eq!(ReportKind::parse("whatever"), Err(ReportKindError));
        assert_eq!(ReportKind::parse(""), Err(ReportKindError));
        assert_eq!(ReportKind::parse("RULE"), Err(ReportKindError));
    }

    #[test]
    fn a_topic_target_carries_no_comment() {
        let target = topic_target();
        assert_eq!(target.topic_id(), TopicId::new(uuid::Uuid::nil()));
        assert_eq!(target.comment_id(), None);
    }

    #[test]
    fn a_comment_target_carries_both_ids() {
        let target = ReportTarget::Comment(
            TopicId::new(uuid::Uuid::nil()),
            CommentId::new(uuid::Uuid::max()),
        );
        assert_eq!(target.topic_id(), TopicId::new(uuid::Uuid::nil()));
        assert_eq!(target.comment_id(), Some(CommentId::new(uuid::Uuid::max())));
    }

    #[test]
    fn a_new_report_is_open() {
        let report = report();
        assert!(report.is_open());
        assert_eq!(report.closed_by(), None);
        assert_eq!(report.closed_at(), None);
        assert_eq!(report.kind(), ReportKind::Rule);
        assert_eq!(report.reason(), &reason());
    }

    #[test]
    fn closing_records_who_and_when_and_keeps_the_rest() {
        let report = report();
        let closer = UserId::new(uuid::Uuid::from_u128(7));
        let closed = report.closed(closer, OffsetDateTime::UNIX_EPOCH);
        assert!(!closed.is_open());
        assert_eq!(closed.closed_by(), Some(closer));
        assert_eq!(closed.closed_at(), Some(OffsetDateTime::UNIX_EPOCH));
        assert_eq!(closed.id(), report.id());
        assert_eq!(closed.reporter_id(), report.reporter_id());
        assert_eq!(closed.target(), report.target());
    }

    #[test]
    fn closing_leaves_the_original_open_report_untouched() {
        let report = report();
        let closed = report.closed(
            UserId::new(uuid::Uuid::from_u128(7)),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(report.is_open());
        assert!(!closed.is_open());
    }
}
