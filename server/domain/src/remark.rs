use crate::UserId;
use time::OffsetDateTime;

pub const MAX_LENGTH: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemarkText(String);

#[derive(Debug, PartialEq, Eq)]
pub enum RemarkTextError {
    Empty,
    TooLong,
}

impl RemarkText {
    pub fn parse(raw: &str) -> Result<Self, RemarkTextError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(RemarkTextError::Empty);
        }
        if trimmed.chars().count() > MAX_LENGTH {
            return Err(RemarkTextError::TooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remark {
    author_id: UserId,
    subject_id: UserId,
    text: RemarkText,
    created_at: OffsetDateTime,
}

impl Remark {
    pub fn new(
        author_id: UserId,
        subject_id: UserId,
        text: RemarkText,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            author_id,
            subject_id,
            text,
            created_at,
        }
    }

    pub fn author_id(&self) -> UserId {
        self.author_id
    }

    pub fn subject_id(&self) -> UserId {
        self.subject_id
    }

    pub fn text(&self) -> &RemarkText {
        &self.text
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn readable_by(&self, reader_id: UserId) -> bool {
        self.author_id == reader_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn author() -> UserId {
        UserId::new(uuid::Uuid::from_u128(1))
    }

    fn subject() -> UserId {
        UserId::new(uuid::Uuid::from_u128(2))
    }

    #[test]
    fn parses_an_ordinary_note() {
        let text = RemarkText::parse("helpful in the tagging thread").unwrap();
        assert_eq!(text.as_str(), "helpful in the tagging thread");
    }

    #[test]
    fn trims_surrounding_space() {
        assert_eq!(RemarkText::parse("  noted  ").unwrap().as_str(), "noted");
    }

    #[test]
    fn refuses_an_empty_note() {
        assert_eq!(RemarkText::parse(""), Err(RemarkTextError::Empty));
        assert_eq!(RemarkText::parse("   "), Err(RemarkTextError::Empty));
    }

    #[test]
    fn refuses_one_that_is_too_long() {
        let long = "a".repeat(MAX_LENGTH + 1);
        assert_eq!(RemarkText::parse(&long), Err(RemarkTextError::TooLong));
    }

    #[test]
    fn accepts_one_exactly_at_the_limit() {
        assert!(RemarkText::parse(&"a".repeat(MAX_LENGTH)).is_ok());
    }

    #[test]
    fn counts_length_in_characters_not_bytes() {
        assert!(RemarkText::parse(&"\u{4e2d}".repeat(MAX_LENGTH)).is_ok());
    }

    #[test]
    fn a_note_is_readable_only_by_the_one_who_wrote_it() {
        let remark = Remark::new(
            author(),
            subject(),
            RemarkText::parse("noted").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(remark.readable_by(author()));
        assert!(!remark.readable_by(subject()));
        assert!(!remark.readable_by(UserId::new(uuid::Uuid::from_u128(3))));
    }

    #[test]
    fn a_note_carries_who_wrote_it_about_whom_and_when() {
        let remark = Remark::new(
            author(),
            subject(),
            RemarkText::parse("noted").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(remark.author_id(), author());
        assert_eq!(remark.subject_id(), subject());
        assert_eq!(remark.text().as_str(), "noted");
        assert_eq!(remark.created_at(), OffsetDateTime::UNIX_EPOCH);
    }
}
