use crate::UserId;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(uuid::Uuid);

impl SessionId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionToken(String);

#[derive(Debug, PartialEq, Eq)]
pub enum SessionTokenError {
    TooShort,
}

impl SessionToken {
    pub fn parse(raw: &str) -> Result<Self, SessionTokenError> {
        let value = raw.trim();
        if value.len() < 16 {
            return Err(SessionTokenError::TooShort);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    id: SessionId,
    user_id: UserId,
    token: SessionToken,
    expires_at: OffsetDateTime,
}

impl Session {
    pub fn new(
        id: SessionId,
        user_id: UserId,
        token: SessionToken,
        expires_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            token,
            expires_at,
        }
    }

    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn token(&self) -> &SessionToken {
        &self.token
    }

    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }

    pub fn is_valid_at(&self, now: OffsetDateTime) -> bool {
        now < self.expires_at
    }

    pub fn extended(&self, new_expiry: OffsetDateTime) -> Self {
        Self {
            expires_at: new_expiry,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn user_id() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn session_id() -> SessionId {
        SessionId::new(uuid::Uuid::nil())
    }

    #[test]
    fn parses_a_token_of_sufficient_length() {
        let token = SessionToken::parse("0123456789abcdef").unwrap();
        assert_eq!(token.as_str(), "0123456789abcdef");
    }

    #[test]
    fn rejects_too_short_token() {
        assert_eq!(
            SessionToken::parse("short"),
            Err(SessionTokenError::TooShort)
        );
    }

    #[test]
    fn is_valid_before_expiry() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let token = SessionToken::parse("0123456789abcdef").unwrap();
        let session = Session::new(session_id(), user_id(), token, now + Duration::days(1));
        assert!(session.is_valid_at(now));
    }

    #[test]
    fn is_invalid_at_or_after_expiry() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let expires_at = now + Duration::days(1);
        let token = SessionToken::parse("0123456789abcdef").unwrap();
        let session = Session::new(session_id(), user_id(), token, expires_at);
        assert!(!session.is_valid_at(expires_at));
        assert!(!session.is_valid_at(expires_at + Duration::seconds(1)));
    }

    #[test]
    fn extended_keeps_identity_and_updates_expiry() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let token = SessionToken::parse("0123456789abcdef").unwrap();
        let session = Session::new(session_id(), user_id(), token, now + Duration::days(1));
        let extended = session.extended(now + Duration::days(30));
        assert_eq!(extended.id(), session.id());
        assert_eq!(extended.expires_at(), now + Duration::days(30));
    }
}
