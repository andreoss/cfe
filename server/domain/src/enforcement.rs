use crate::{Address, Reason, UserId};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockMode {
    Refuse,
    Challenge,
    Allow,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockModeError;

impl BlockMode {
    pub fn parse(raw: &str) -> Result<Self, BlockModeError> {
        match raw {
            "refuse" => Ok(Self::Refuse),
            "challenge" => Ok(Self::Challenge),
            "allow" => Ok(Self::Allow),
            _ => Err(BlockModeError),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Refuse => "refuse",
            Self::Challenge => "challenge",
            Self::Allow => "allow",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Refuse, Self::Challenge, Self::Allow]
    }
}

impl Default for BlockMode {
    fn default() -> Self {
        Self::Refuse
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressBlock {
    addr: Address,
    moderator_id: UserId,
    reason: Reason,
    blocked_at: OffsetDateTime,
    until: Option<OffsetDateTime>,
    mode: BlockMode,
}

impl AddressBlock {
    pub fn new(
        addr: Address,
        moderator_id: UserId,
        reason: Reason,
        blocked_at: OffsetDateTime,
        until: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            addr,
            moderator_id,
            reason,
            blocked_at,
            until,
            mode: BlockMode::Refuse,
        }
    }

    pub fn with_mode(&self, mode: BlockMode) -> Self {
        Self {
            addr: self.addr.clone(),
            moderator_id: self.moderator_id,
            reason: self.reason.clone(),
            blocked_at: self.blocked_at,
            until: self.until,
            mode,
        }
    }

    pub fn mode(&self) -> BlockMode {
        self.mode
    }

    pub fn addr(&self) -> &Address {
        &self.addr
    }

    pub fn moderator_id(&self) -> UserId {
        self.moderator_id
    }

    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    pub fn blocked_at(&self) -> OffsetDateTime {
        self.blocked_at
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

    #[test]
    fn an_address_block_without_an_end_is_always_active() {
        let block = AddressBlock::new(
            crate::Address::parse("203.0.113.9").unwrap(),
            moderator(),
            reason(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        );
        assert!(block.is_active_at(OffsetDateTime::UNIX_EPOCH));
        assert!(block.is_active_at(OffsetDateTime::UNIX_EPOCH + Duration::days(3650)));
        assert_eq!(block.addr().as_str(), "203.0.113.9");
        assert_eq!(block.until(), None);
        assert_eq!(block.reason(), &reason());
        assert_eq!(block.moderator_id(), moderator());
    }

    #[test]
    fn a_timed_address_block_lapses_at_its_end() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let until = start + Duration::days(30);
        let block = AddressBlock::new(
            crate::Address::parse("198.51.100.1/24").unwrap(),
            moderator(),
            reason(),
            start,
            Some(until),
        );
        assert!(block.is_active_at(start));
        assert!(block.is_active_at(until - Duration::seconds(1)));
        assert!(!block.is_active_at(until));
        assert!(!block.is_active_at(until + Duration::days(1)));
    }
}

#[cfg(test)]
mod block_mode_tests {
    use super::*;

    #[test]
    fn parses_every_mode_round_trip() {
        for mode in BlockMode::all() {
            assert_eq!(BlockMode::parse(mode.as_str()), Ok(mode));
        }
    }

    #[test]
    fn rejects_an_unknown_mode() {
        assert_eq!(BlockMode::parse("maybe"), Err(BlockModeError));
        assert_eq!(BlockMode::parse(""), Err(BlockModeError));
    }

    #[test]
    fn a_block_refuses_unless_told_otherwise() {
        let block = AddressBlock::new(
            Address::parse("203.0.113.10").unwrap(),
            UserId::new(uuid::Uuid::nil()),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        );
        assert_eq!(block.mode(), BlockMode::Refuse);
        assert_eq!(BlockMode::default(), BlockMode::Refuse);
    }

    #[test]
    fn a_mode_can_be_set_without_disturbing_the_rest() {
        let block = AddressBlock::new(
            Address::parse("203.0.113.10").unwrap(),
            UserId::new(uuid::Uuid::nil()),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
            None,
        );
        let challenged = block.with_mode(BlockMode::Challenge);
        assert_eq!(challenged.mode(), BlockMode::Challenge);
        assert_eq!(challenged.addr(), block.addr());
        assert_eq!(challenged.reason(), block.reason());
        assert_eq!(challenged.until(), block.until());
        assert!(challenged.is_active_at(OffsetDateTime::UNIX_EPOCH));
    }
}
