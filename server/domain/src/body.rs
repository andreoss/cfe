#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Body(String);

#[derive(Debug, PartialEq, Eq)]
pub enum BodyError {
    Empty,
    TooLong,
}

const MAX_LEN: usize = 10_000;

impl Body {
    pub fn parse(raw: &str) -> Result<Self, BodyError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(BodyError::Empty);
        }
        if value.chars().count() > MAX_LEN {
            return Err(BodyError::TooLong);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_body() {
        let body = Body::parse("Hello, world.").unwrap();
        assert_eq!(body.as_str(), "Hello, world.");
    }

    #[test]
    fn rejects_empty_or_blank() {
        assert_eq!(Body::parse(""), Err(BodyError::Empty));
        assert_eq!(Body::parse("   "), Err(BodyError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(MAX_LEN + 1);
        assert_eq!(Body::parse(&raw), Err(BodyError::TooLong));
    }

    #[test]
    fn accepts_exactly_max_len() {
        let raw = "a".repeat(MAX_LEN);
        assert!(Body::parse(&raw).is_ok());
    }
}
