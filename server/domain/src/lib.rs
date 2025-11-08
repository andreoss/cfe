#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Username(String);

#[derive(Debug, PartialEq, Eq)]
pub enum UsernameError {
    TooShort,
    TooLong,
    InvalidStart,
    InvalidChar(char),
}

impl Username {
    pub fn parse(raw: &str) -> Result<Self, UsernameError> {
        let value = raw.trim();
        if value.chars().count() < 3 {
            return Err(UsernameError::TooShort);
        }
        if value.chars().count() > 32 {
            return Err(UsernameError::TooLong);
        }
        let first = value.chars().next().ok_or(UsernameError::TooShort)?;
        if !first.is_ascii_alphabetic() {
            return Err(UsernameError::InvalidStart);
        }
        match value
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-'))
        {
            Some(c) => Err(UsernameError::InvalidChar(c)),
            None => Ok(Self(value.to_owned())),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_name() {
        let name = Username::parse("alice_01").unwrap();
        assert_eq!(name.as_str(), "alice_01");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let name = Username::parse("  bob  ").unwrap();
        assert_eq!(name.as_str(), "bob");
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(Username::parse("ab"), Err(UsernameError::TooShort));
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(33);
        assert_eq!(Username::parse(&raw), Err(UsernameError::TooLong));
    }

    #[test]
    fn rejects_leading_digit() {
        assert_eq!(Username::parse("1abc"), Err(UsernameError::InvalidStart));
    }

    #[test]
    fn rejects_invalid_character() {
        assert_eq!(
            Username::parse("ali ce"),
            Err(UsernameError::InvalidChar(' '))
        );
    }
}
