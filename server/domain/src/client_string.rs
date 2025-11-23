pub const MAX_LENGTH: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientString(String);

#[derive(Debug, PartialEq, Eq)]
pub enum ClientStringError {
    Empty,
    TooLong,
}

impl ClientString {
    pub fn parse(raw: &str) -> Result<Self, ClientStringError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ClientStringError::Empty);
        }
        if trimmed.chars().count() > MAX_LENGTH {
            return Err(ClientStringError::TooLong);
        }
        Ok(Self(trimmed.chars().filter(|c| !c.is_control()).collect()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_ordinary_client_string() {
        let parsed = ClientString::parse("Mozilla/5.0 (X11; Linux x86_64)").unwrap();
        assert_eq!(parsed.as_str(), "Mozilla/5.0 (X11; Linux x86_64)");
    }

    #[test]
    fn trims_surrounding_space() {
        let parsed = ClientString::parse("  something/1.0  ").unwrap();
        assert_eq!(parsed.as_str(), "something/1.0");
    }

    #[test]
    fn refuses_an_empty_string() {
        assert_eq!(ClientString::parse(""), Err(ClientStringError::Empty));
        assert_eq!(ClientString::parse("   "), Err(ClientStringError::Empty));
    }

    #[test]
    fn refuses_one_that_is_too_long() {
        let long = "a".repeat(MAX_LENGTH + 1);
        assert_eq!(ClientString::parse(&long), Err(ClientStringError::TooLong));
    }

    #[test]
    fn accepts_one_exactly_at_the_limit() {
        let limit = "a".repeat(MAX_LENGTH);
        assert!(ClientString::parse(&limit).is_ok());
    }

    #[test]
    fn strips_control_characters() {
        let parsed = ClientString::parse("agent\u{0}/1.0\u{7}").unwrap();
        assert_eq!(parsed.as_str(), "agent/1.0");
    }

    #[test]
    fn counts_length_in_characters_not_bytes() {
        let wide = "\u{4e2d}".repeat(MAX_LENGTH);
        assert!(ClientString::parse(&wide).is_ok());
    }
}
