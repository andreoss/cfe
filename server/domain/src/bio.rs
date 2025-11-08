#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bio(String);

#[derive(Debug, PartialEq, Eq)]
pub enum BioError {
    TooLong,
}

const MAX_LEN: usize = 500;

impl Bio {
    pub fn parse(raw: &str) -> Result<Option<Self>, BioError> {
        let value = raw.trim();
        if value.is_empty() {
            return Ok(None);
        }
        if value.chars().count() > MAX_LEN {
            return Err(BioError::TooLong);
        }
        Ok(Some(Self(value.to_owned())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_non_empty_bio() {
        let bio = Bio::parse("Rust and forums.").unwrap().unwrap();
        assert_eq!(bio.as_str(), "Rust and forums.");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let bio = Bio::parse("  hello  ").unwrap().unwrap();
        assert_eq!(bio.as_str(), "hello");
    }

    #[test]
    fn empty_or_blank_parses_to_none() {
        assert_eq!(Bio::parse("").unwrap(), None);
        assert_eq!(Bio::parse("   ").unwrap(), None);
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(MAX_LEN + 1);
        assert_eq!(Bio::parse(&raw), Err(BioError::TooLong));
    }

    #[test]
    fn accepts_exactly_max_len() {
        let raw = "a".repeat(MAX_LEN);
        assert!(Bio::parse(&raw).unwrap().is_some());
    }
}
