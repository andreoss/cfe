use crate::UserId;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenPurpose {
    Activation,
    PasswordReset,
    EmailChange,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TokenPurposeError;

impl TokenPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Activation => "activation",
            Self::PasswordReset => "password_reset",
            Self::EmailChange => "email_change",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, TokenPurposeError> {
        match raw {
            "activation" => Ok(Self::Activation),
            "password_reset" => Ok(Self::PasswordReset),
            "email_change" => Ok(Self::EmailChange),
            _ => Err(TokenPurposeError),
        }
    }

    pub fn lifetime(&self) -> Duration {
        match self {
            Self::Activation => Duration::days(1),
            Self::PasswordReset | Self::EmailChange => Duration::hours(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailToken {
    id: MailTokenId,
    user_id: UserId,
    purpose: TokenPurpose,
    digest: String,
    payload: Option<String>,
    expires_at: OffsetDateTime,
    redeemed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MailTokenId(uuid::Uuid);

impl MailTokenId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl MailToken {
    pub fn issue(
        id: MailTokenId,
        user_id: UserId,
        purpose: TokenPurpose,
        digest: String,
        payload: Option<String>,
        now: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            purpose,
            digest,
            payload,
            expires_at: now + purpose.lifetime(),
            redeemed_at: None,
        }
    }

    pub fn from_parts(
        id: MailTokenId,
        user_id: UserId,
        purpose: TokenPurpose,
        digest: String,
        payload: Option<String>,
        expires_at: OffsetDateTime,
        redeemed_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            user_id,
            purpose,
            digest,
            payload,
            expires_at,
            redeemed_at,
        }
    }

    pub fn id(&self) -> MailTokenId {
        self.id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn purpose(&self) -> TokenPurpose {
        self.purpose
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn payload(&self) -> Option<&str> {
        self.payload.as_deref()
    }

    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }

    pub fn redeemed_at(&self) -> Option<OffsetDateTime> {
        self.redeemed_at
    }

    pub fn is_redeemable_at(&self, now: OffsetDateTime) -> bool {
        self.redeemed_at.is_none() && now < self.expires_at
    }

    pub fn redeemed(&self, at: OffsetDateTime) -> Self {
        Self {
            redeemed_at: Some(at),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(purpose: TokenPurpose) -> MailToken {
        MailToken::issue(
            MailTokenId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            purpose,
            "digest".to_owned(),
            None,
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn every_purpose_round_trips_through_its_name() {
        for purpose in [
            TokenPurpose::Activation,
            TokenPurpose::PasswordReset,
            TokenPurpose::EmailChange,
        ] {
            assert_eq!(TokenPurpose::parse(purpose.as_str()), Ok(purpose));
        }
        assert_eq!(TokenPurpose::parse("something"), Err(TokenPurposeError));
    }

    #[test]
    fn a_reset_expires_sooner_than_an_activation() {
        assert_eq!(TokenPurpose::PasswordReset.lifetime(), Duration::hours(1));
        assert_eq!(TokenPurpose::EmailChange.lifetime(), Duration::hours(1));
        assert_eq!(TokenPurpose::Activation.lifetime(), Duration::days(1));
    }

    #[test]
    fn a_fresh_token_is_redeemable_until_it_expires() {
        let token = token(TokenPurpose::PasswordReset);
        assert!(token.is_redeemable_at(OffsetDateTime::UNIX_EPOCH));
        assert!(token.is_redeemable_at(OffsetDateTime::UNIX_EPOCH + Duration::minutes(59)));
        assert!(!token.is_redeemable_at(OffsetDateTime::UNIX_EPOCH + Duration::hours(1)));
        assert!(!token.is_redeemable_at(OffsetDateTime::UNIX_EPOCH + Duration::days(1)));
    }

    #[test]
    fn a_redeemed_token_cannot_be_used_again() {
        let token = token(TokenPurpose::PasswordReset);
        let used = token.redeemed(OffsetDateTime::UNIX_EPOCH);
        assert_eq!(used.id(), token.id());
        assert_eq!(used.redeemed_at(), Some(OffsetDateTime::UNIX_EPOCH));
        assert!(!used.is_redeemable_at(OffsetDateTime::UNIX_EPOCH));
    }

    #[test]
    fn only_the_digest_is_carried_never_the_secret() {
        let token = token(TokenPurpose::PasswordReset);
        assert_eq!(token.digest(), "digest");
        assert_eq!(token.payload(), None);
    }

    #[test]
    fn an_address_change_carries_the_new_address_as_its_payload() {
        let token = MailToken::issue(
            MailTokenId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            TokenPurpose::EmailChange,
            "digest".to_owned(),
            Some("new@example.com".to_owned()),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(token.payload(), Some("new@example.com"));
    }
}
