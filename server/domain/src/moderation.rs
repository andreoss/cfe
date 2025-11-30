use crate::{Penalty, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reason(String);

#[derive(Debug, PartialEq, Eq)]
pub enum ReasonError {
    Empty,
    TooLong,
}

const MAX_LEN: usize = 500;

impl Reason {
    pub fn parse(raw: &str) -> Result<Self, ReasonError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(ReasonError::Empty);
        }
        if value.chars().count() > MAX_LEN {
            return Err(ReasonError::TooLong);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deletion {
    moderator_id: UserId,
    reason: Reason,
    penalty: Penalty,
    deleted_at: OffsetDateTime,
}

impl Deletion {
    pub fn new(
        moderator_id: UserId,
        reason: Reason,
        penalty: Penalty,
        deleted_at: OffsetDateTime,
    ) -> Self {
        Self {
            moderator_id,
            reason,
            penalty,
            deleted_at,
        }
    }

    pub fn penalty(&self) -> Penalty {
        self.penalty
    }

    pub fn moderator_id(&self) -> UserId {
        self.moderator_id
    }

    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    pub fn deleted_at(&self) -> OffsetDateTime {
        self.deleted_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Penalty;

    #[test]
    fn parses_a_valid_reason() {
        let reason = Reason::parse("off-topic").unwrap();
        assert_eq!(reason.as_str(), "off-topic");
    }

    #[test]
    fn rejects_empty_or_blank() {
        assert_eq!(Reason::parse(""), Err(ReasonError::Empty));
        assert_eq!(Reason::parse("   "), Err(ReasonError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(MAX_LEN + 1);
        assert_eq!(Reason::parse(&raw), Err(ReasonError::TooLong));
    }

    #[test]
    fn deletion_exposes_its_fields() {
        let moderator_id = UserId::new(uuid::Uuid::nil());
        let reason = Reason::parse("spam").unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let deletion = Deletion::new(moderator_id, reason.clone(), Penalty::default(), now);
        assert_eq!(deletion.moderator_id(), moderator_id);
        assert_eq!(deletion.reason(), &reason);
        assert_eq!(deletion.deleted_at(), now);
    }
}
