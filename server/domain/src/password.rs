#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Password(String);

#[derive(Debug, PartialEq, Eq)]
pub enum PasswordError {
    TooShort,
    TooLong,
}

const MIN_LEN: usize = 8;
const MAX_LEN: usize = 200;

impl Password {
    pub fn parse(raw: &str) -> Result<Self, PasswordError> {
        let len = raw.chars().count();
        if len < MIN_LEN {
            return Err(PasswordError::TooShort);
        }
        if len > MAX_LEN {
            return Err(PasswordError::TooLong);
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_long_enough_password() {
        assert_eq!(
            Password::parse("correcthorse").unwrap().as_str(),
            "correcthorse"
        );
    }

    #[test]
    fn rejects_anything_shorter_than_the_minimum() {
        assert_eq!(Password::parse(""), Err(PasswordError::TooShort));
        assert_eq!(Password::parse("short"), Err(PasswordError::TooShort));
        assert_eq!(
            Password::parse(&"a".repeat(MIN_LEN - 1)),
            Err(PasswordError::TooShort)
        );
    }

    #[test]
    fn accepts_exactly_the_minimum() {
        assert!(Password::parse(&"a".repeat(MIN_LEN)).is_ok());
    }

    #[test]
    fn rejects_an_absurdly_long_password() {
        assert_eq!(
            Password::parse(&"a".repeat(MAX_LEN + 1)),
            Err(PasswordError::TooLong)
        );
    }

    #[test]
    fn keeps_surrounding_whitespace_rather_than_trimming_it() {
        let raw = "  spaces  ";
        assert_eq!(Password::parse(raw).unwrap().as_str(), raw);
    }

    #[test]
    fn counts_characters_not_bytes() {
        let eight_chars = "héllo wörld";
        assert!(Password::parse(eight_chars).is_ok());
        assert_eq!(Password::parse("héllo"), Err(PasswordError::TooShort));
    }
}
