#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

#[derive(Debug, PartialEq, Eq)]
pub enum EmailError {
    Empty,
    MissingAt,
    EmptyLocalPart,
    EmptyDomain,
    DomainMissingDot,
}

impl Email {
    pub fn parse(raw: &str) -> Result<Self, EmailError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(EmailError::Empty);
        }
        let (local, domain) = value.split_once('@').ok_or(EmailError::MissingAt)?;
        if local.is_empty() {
            return Err(EmailError::EmptyLocalPart);
        }
        if domain.is_empty() {
            return Err(EmailError::EmptyDomain);
        }
        if !domain.contains('.') {
            return Err(EmailError::DomainMissingDot);
        }
        Ok(Self(value.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_address() {
        let email = Email::parse("Alice@Example.com").unwrap();
        assert_eq!(email.as_str(), "alice@example.com");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(Email::parse(""), Err(EmailError::Empty));
    }

    #[test]
    fn rejects_missing_at() {
        assert_eq!(
            Email::parse("alice.example.com"),
            Err(EmailError::MissingAt)
        );
    }

    #[test]
    fn rejects_empty_local_part() {
        assert_eq!(
            Email::parse("@example.com"),
            Err(EmailError::EmptyLocalPart)
        );
    }

    #[test]
    fn rejects_empty_domain() {
        assert_eq!(Email::parse("alice@"), Err(EmailError::EmptyDomain));
    }

    #[test]
    fn rejects_domain_without_dot() {
        assert_eq!(
            Email::parse("alice@localhost"),
            Err(EmailError::DomainMissingDot)
        );
    }
}
