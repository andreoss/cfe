#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Title(String);

#[derive(Debug, PartialEq, Eq)]
pub enum TitleError {
    Empty,
    TooLong,
}

const MAX_LEN: usize = 200;

impl Title {
    pub fn parse(raw: &str) -> Result<Self, TitleError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(TitleError::Empty);
        }
        if value.chars().count() > MAX_LEN {
            return Err(TitleError::TooLong);
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
    fn accepts_a_valid_title() {
        let title = Title::parse("Hello, forum!").unwrap();
        assert_eq!(title.as_str(), "Hello, forum!");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let title = Title::parse("  hi  ").unwrap();
        assert_eq!(title.as_str(), "hi");
    }

    #[test]
    fn rejects_empty_or_blank() {
        assert_eq!(Title::parse(""), Err(TitleError::Empty));
        assert_eq!(Title::parse("   "), Err(TitleError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(MAX_LEN + 1);
        assert_eq!(Title::parse(&raw), Err(TitleError::TooLong));
    }

    #[test]
    fn accepts_exactly_max_len() {
        let raw = "a".repeat(MAX_LEN);
        assert!(Title::parse(&raw).is_ok());
    }
}
