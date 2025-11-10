#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query(String);

#[derive(Debug, PartialEq, Eq)]
pub enum QueryError {
    Empty,
    TooLong,
}

const MAX_LEN: usize = 200;

impl Query {
    pub fn parse(raw: &str) -> Result<Self, QueryError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(QueryError::Empty);
        }
        if value.chars().count() > MAX_LEN {
            return Err(QueryError::TooLong);
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
    fn parses_a_non_empty_query() {
        let query = Query::parse("rust forum").unwrap();
        assert_eq!(query.as_str(), "rust forum");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let query = Query::parse("  rust  ").unwrap();
        assert_eq!(query.as_str(), "rust");
    }

    #[test]
    fn rejects_empty_or_blank() {
        assert_eq!(Query::parse(""), Err(QueryError::Empty));
        assert_eq!(Query::parse("   "), Err(QueryError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(MAX_LEN + 1);
        assert_eq!(Query::parse(&raw), Err(QueryError::TooLong));
    }

    #[test]
    fn accepts_exactly_max_len() {
        let raw = "a".repeat(MAX_LEN);
        assert!(Query::parse(&raw).is_ok());
    }
}
