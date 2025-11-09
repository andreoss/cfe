#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Slug(String);

#[derive(Debug, PartialEq, Eq)]
pub enum SlugError {
    TooShort,
    TooLong,
    InvalidChar(char),
}

impl Slug {
    pub fn parse(raw: &str) -> Result<Self, SlugError> {
        let value = raw.trim();
        if value.chars().count() < 2 {
            return Err(SlugError::TooShort);
        }
        if value.chars().count() > 50 {
            return Err(SlugError::TooLong);
        }
        match value
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'))
        {
            Some(c) => Err(SlugError::InvalidChar(c)),
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
    fn accepts_a_valid_slug() {
        let slug = Slug::parse("general-talk").unwrap();
        assert_eq!(slug.as_str(), "general-talk");
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(Slug::parse("a"), Err(SlugError::TooShort));
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(51);
        assert_eq!(Slug::parse(&raw), Err(SlugError::TooLong));
    }

    #[test]
    fn rejects_uppercase() {
        assert_eq!(Slug::parse("Talk"), Err(SlugError::InvalidChar('T')));
    }

    #[test]
    fn rejects_spaces() {
        assert_eq!(
            Slug::parse("general talk"),
            Err(SlugError::InvalidChar(' '))
        );
    }
}
