use crate::{Reason, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ban {
    moderator_id: UserId,
    reason: Reason,
    banned_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
}

impl Ban {
    pub fn new(
        moderator_id: UserId,
        reason: Reason,
        banned_at: OffsetDateTime,
        until: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            moderator_id,
            reason,
            banned_at,
            until,
        }
    }

    pub fn moderator_id(&self) -> UserId {
        self.moderator_id
    }

    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    pub fn banned_at(&self) -> OffsetDateTime {
        self.banned_at
    }

    pub fn until(&self) -> Option<OffsetDateTime> {
        self.until
    }

    pub fn is_active_at(&self, now: OffsetDateTime) -> bool {
        match self.until {
            Some(until) => now < until,
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    id: WarningId,
    user_id: UserId,
    moderator_id: UserId,
    reason: Reason,
    created_at: OffsetDateTime,
    acknowledged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WarningId(uuid::Uuid);

impl WarningId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Warning {
    pub fn new(
        id: WarningId,
        user_id: UserId,
        moderator_id: UserId,
        reason: Reason,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            moderator_id,
            reason,
            created_at,
            acknowledged: false,
        }
    }

    pub fn from_parts(
        id: WarningId,
        user_id: UserId,
        moderator_id: UserId,
        reason: Reason,
        created_at: OffsetDateTime,
        acknowledged: bool,
    ) -> Self {
        Self {
            id,
            user_id,
            moderator_id,
            reason,
            created_at,
            acknowledged,
        }
    }

    pub fn id(&self) -> WarningId {
        self.id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn moderator_id(&self) -> UserId {
        self.moderator_id
    }

    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn is_acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub fn acknowledged(&self) -> Self {
        Self {
            acknowledged: true,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn reason() -> Reason {
        Reason::parse("repeated spam").unwrap()
    }

    fn moderator() -> UserId {
        UserId::new(uuid::Uuid::max())
    }

    #[test]
    fn a_ban_without_an_end_is_always_active() {
        let ban = Ban::new(moderator(), reason(), OffsetDateTime::UNIX_EPOCH, None);
        assert!(ban.is_active_at(OffsetDateTime::UNIX_EPOCH));
        assert!(ban.is_active_at(OffsetDateTime::UNIX_EPOCH + Duration::days(3650)));
        assert_eq!(ban.until(), None);
        assert_eq!(ban.reason(), &reason());
        assert_eq!(ban.moderator_id(), moderator());
    }

    #[test]
    fn a_timed_ban_lapses_at_its_end() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let until = start + Duration::days(7);
        let ban = Ban::new(moderator(), reason(), start, Some(until));
        assert!(ban.is_active_at(start));
        assert!(ban.is_active_at(until - Duration::seconds(1)));
        assert!(!ban.is_active_at(until));
        assert!(!ban.is_active_at(until + Duration::days(1)));
    }

    #[test]
    fn a_warning_starts_unacknowledged() {
        let warning = Warning::new(
            WarningId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            moderator(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(!warning.is_acknowledged());
        assert_eq!(warning.reason(), &reason());
        assert_eq!(warning.moderator_id(), moderator());
    }

    #[test]
    fn acknowledging_keeps_identity() {
        let warning = Warning::new(
            WarningId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            moderator(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let seen = warning.acknowledged();
        assert_eq!(seen.id(), warning.id());
        assert_eq!(seen.user_id(), warning.user_id());
        assert_eq!(seen.created_at(), warning.created_at());
        assert!(seen.is_acknowledged());
    }
}
