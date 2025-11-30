use crate::UserId;
use time::{Duration, OffsetDateTime};

pub const CODE_LENGTH: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvitationCode(String);

#[derive(Debug, PartialEq, Eq)]
pub enum InvitationCodeError {
    WrongLength,
    NotAllowedCharacter,
}

impl InvitationCode {
    pub fn parse(raw: &str) -> Result<Self, InvitationCodeError> {
        let trimmed = raw.trim().to_ascii_uppercase();
        if trimmed.chars().count() != CODE_LENGTH {
            return Err(InvitationCodeError::WrongLength);
        }
        if !trimmed.chars().all(|c| ALPHABET.contains(c)) {
            return Err(InvitationCodeError::NotAllowedCharacter);
        }
        Ok(Self(trimmed))
    }

    pub fn from_bytes(bytes: &[u8; CODE_LENGTH]) -> Self {
        let letters: Vec<char> = ALPHABET.chars().collect();
        let code = bytes
            .iter()
            .map(|b| letters[*b as usize % letters.len()])
            .collect();
        Self(code)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvitationId(uuid::Uuid);

impl InvitationId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    id: InvitationId,
    code: InvitationCode,
    issuer_id: UserId,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
    spent_at: Option<OffsetDateTime>,
    spent_by: Option<UserId>,
}

impl Invitation {
    pub fn issue(
        id: InvitationId,
        code: InvitationCode,
        issuer_id: UserId,
        now: OffsetDateTime,
        lifetime: Duration,
    ) -> Self {
        Self {
            id,
            code,
            issuer_id,
            created_at: now,
            expires_at: now + lifetime,
            spent_at: None,
            spent_by: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: InvitationId,
        code: InvitationCode,
        issuer_id: UserId,
        created_at: OffsetDateTime,
        expires_at: OffsetDateTime,
        spent_at: Option<OffsetDateTime>,
        spent_by: Option<UserId>,
    ) -> Self {
        Self {
            id,
            code,
            issuer_id,
            created_at,
            expires_at,
            spent_at,
            spent_by,
        }
    }

    pub fn id(&self) -> InvitationId {
        self.id
    }

    pub fn code(&self) -> &InvitationCode {
        &self.code
    }

    pub fn issuer_id(&self) -> UserId {
        self.issuer_id
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }

    pub fn spent_at(&self) -> Option<OffsetDateTime> {
        self.spent_at
    }

    pub fn spent_by(&self) -> Option<UserId> {
        self.spent_by
    }

    pub fn is_spent(&self) -> bool {
        self.spent_at.is_some()
    }

    pub fn is_spendable_at(&self, now: OffsetDateTime) -> bool {
        self.spent_at.is_none() && now < self.expires_at
    }

    pub fn spent(&self, by: UserId, at: OffsetDateTime) -> Self {
        Self {
            spent_at: Some(at),
            spent_by: Some(by),
            ..self.clone()
        }
    }

    pub fn claimed(&self, at: OffsetDateTime) -> Self {
        Self {
            spent_at: Some(at),
            ..self.clone()
        }
    }

    pub fn attributed(&self, by: UserId) -> Self {
        Self {
            spent_by: Some(by),
            ..self.clone()
        }
    }

    pub fn released(&self) -> Self {
        Self {
            spent_at: None,
            spent_by: None,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issuer() -> UserId {
        UserId::new(uuid::Uuid::from_u128(1))
    }

    fn invitee() -> UserId {
        UserId::new(uuid::Uuid::from_u128(2))
    }

    fn code() -> InvitationCode {
        InvitationCode::parse("ABCDEFGHJKLM").unwrap()
    }

    fn invitation() -> Invitation {
        Invitation::issue(
            InvitationId::new(uuid::Uuid::nil()),
            code(),
            issuer(),
            OffsetDateTime::UNIX_EPOCH,
            Duration::days(7),
        )
    }

    #[test]
    fn parses_a_code_of_the_expected_shape() {
        assert_eq!(code().as_str(), "ABCDEFGHJKLM");
    }

    #[test]
    fn reads_a_code_however_it_was_typed() {
        assert_eq!(
            InvitationCode::parse("  abcdefghjklm  ").unwrap().as_str(),
            "ABCDEFGHJKLM"
        );
    }

    #[test]
    fn refuses_a_code_of_the_wrong_length() {
        assert_eq!(
            InvitationCode::parse("ABCD"),
            Err(InvitationCodeError::WrongLength)
        );
        assert_eq!(
            InvitationCode::parse("ABCDEFGHJKLMN"),
            Err(InvitationCodeError::WrongLength)
        );
        assert_eq!(
            InvitationCode::parse(""),
            Err(InvitationCodeError::WrongLength)
        );
    }

    #[test]
    fn refuses_a_character_outside_the_alphabet() {
        assert_eq!(
            InvitationCode::parse("ABCDEFGHJKL-"),
            Err(InvitationCodeError::NotAllowedCharacter)
        );
    }

    #[test]
    fn leaves_out_the_letters_that_are_read_as_digits() {
        for confusable in ['I', 'O', '0', '1'] {
            assert!(!ALPHABET.contains(confusable));
        }
    }

    #[test]
    fn builds_a_code_of_the_right_shape_from_bytes() {
        let built = InvitationCode::from_bytes(&[3, 9, 17, 25, 31, 4, 12, 20, 28, 1, 7, 15]);
        assert_eq!(built.as_str().chars().count(), CODE_LENGTH);
        assert_eq!(InvitationCode::parse(built.as_str()), Ok(built));
    }

    #[test]
    fn different_bytes_build_different_codes() {
        let one = InvitationCode::from_bytes(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        let two = InvitationCode::from_bytes(&[12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
        assert_ne!(one, two);
    }

    #[test]
    fn a_fresh_invitation_can_be_spent_until_it_expires() {
        let invitation = invitation();
        assert!(invitation.is_spendable_at(OffsetDateTime::UNIX_EPOCH));
        assert!(invitation.is_spendable_at(OffsetDateTime::UNIX_EPOCH + Duration::days(6)));
        assert!(!invitation.is_spendable_at(OffsetDateTime::UNIX_EPOCH + Duration::days(7)));
    }

    #[test]
    fn spending_it_records_who_took_it_and_when() {
        let spent = invitation().spent(invitee(), OffsetDateTime::UNIX_EPOCH + Duration::days(1));
        assert_eq!(spent.spent_by(), Some(invitee()));
        assert_eq!(
            spent.spent_at(),
            Some(OffsetDateTime::UNIX_EPOCH + Duration::days(1))
        );
        assert!(spent.is_spent());
    }

    #[test]
    fn a_spent_invitation_cannot_be_spent_again() {
        let spent = invitation().spent(invitee(), OffsetDateTime::UNIX_EPOCH);
        assert!(!spent.is_spendable_at(OffsetDateTime::UNIX_EPOCH));
    }

    #[test]
    fn a_claim_holds_it_before_anyone_is_named() {
        let claimed = invitation().claimed(OffsetDateTime::UNIX_EPOCH);
        assert!(claimed.is_spent());
        assert!(!claimed.is_spendable_at(OffsetDateTime::UNIX_EPOCH));
        assert_eq!(claimed.spent_by(), None);
    }

    #[test]
    fn naming_the_taker_completes_a_claim() {
        let taken = invitation()
            .claimed(OffsetDateTime::UNIX_EPOCH)
            .attributed(invitee());
        assert_eq!(taken.spent_by(), Some(invitee()));
        assert_eq!(taken.spent_at(), Some(OffsetDateTime::UNIX_EPOCH));
    }

    #[test]
    fn releasing_a_claim_makes_it_spendable_again() {
        let released = invitation().claimed(OffsetDateTime::UNIX_EPOCH).released();
        assert!(!released.is_spent());
        assert!(released.is_spendable_at(OffsetDateTime::UNIX_EPOCH));
        assert_eq!(released.spent_by(), None);
    }

    #[test]
    fn an_invitation_carries_who_issued_it_and_when() {
        let invitation = invitation();
        assert_eq!(invitation.issuer_id(), issuer());
        assert_eq!(invitation.created_at(), OffsetDateTime::UNIX_EPOCH);
        assert_eq!(
            invitation.expires_at(),
            OffsetDateTime::UNIX_EPOCH + Duration::days(7)
        );
        assert!(!invitation.is_spent());
        assert_eq!(invitation.spent_by(), None);
    }

    #[test]
    fn the_lifetime_is_whatever_it_was_issued_with() {
        let brief = Invitation::issue(
            InvitationId::new(uuid::Uuid::nil()),
            code(),
            issuer(),
            OffsetDateTime::UNIX_EPOCH,
            Duration::hours(1),
        );
        assert!(!brief.is_spendable_at(OffsetDateTime::UNIX_EPOCH + Duration::hours(1)));
    }

    #[test]
    fn it_is_rebuilt_from_its_stored_parts() {
        let stored = Invitation::from_parts(
            InvitationId::new(uuid::Uuid::nil()),
            code(),
            issuer(),
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + Duration::days(7),
            Some(OffsetDateTime::UNIX_EPOCH + Duration::days(1)),
            Some(invitee()),
        );
        assert_eq!(stored.id().as_uuid(), uuid::Uuid::nil());
        assert_eq!(stored.code(), &code());
        assert!(stored.is_spent());
        assert_eq!(stored.spent_by(), Some(invitee()));
    }
}
