use crate::{TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollId(uuid::Uuid);

impl PollId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollOptionId(uuid::Uuid);

impl PollOptionId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question(String);

#[derive(Debug, PartialEq, Eq)]
pub enum QuestionError {
    Empty,
    TooLong,
}

const QUESTION_MAX_LEN: usize = 200;

impl Question {
    pub fn parse(raw: &str) -> Result<Self, QuestionError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(QuestionError::Empty);
        }
        if value.chars().count() > QUESTION_MAX_LEN {
            return Err(QuestionError::TooLong);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollOption {
    id: PollOptionId,
    text: Question,
}

impl PollOption {
    pub fn new(id: PollOptionId, text: Question) -> Self {
        Self { id, text }
    }

    pub fn id(&self) -> PollOptionId {
        self.id
    }

    pub fn text(&self) -> &Question {
        &self.text
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PollError {
    TooFewOptions,
    TooManyOptions,
}

const MIN_OPTIONS: usize = 2;
const MAX_OPTIONS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poll {
    id: PollId,
    topic_id: TopicId,
    question: Question,
    options: Vec<PollOption>,
    created_at: OffsetDateTime,
}

impl Poll {
    pub fn new(
        id: PollId,
        topic_id: TopicId,
        question: Question,
        options: Vec<PollOption>,
        created_at: OffsetDateTime,
    ) -> Result<Self, PollError> {
        if options.len() < MIN_OPTIONS {
            return Err(PollError::TooFewOptions);
        }
        if options.len() > MAX_OPTIONS {
            return Err(PollError::TooManyOptions);
        }
        Ok(Self {
            id,
            topic_id,
            question,
            options,
            created_at,
        })
    }

    pub fn id(&self) -> PollId {
        self.id
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn question(&self) -> &Question {
        &self.question
    }

    pub fn options(&self) -> &[PollOption] {
        &self.options
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn has_option(&self, id: PollOptionId) -> bool {
        self.options.iter().any(|o| o.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vote {
    poll_id: PollId,
    option_id: PollOptionId,
    user_id: UserId,
    created_at: OffsetDateTime,
}

impl Vote {
    pub fn new(
        poll_id: PollId,
        option_id: PollOptionId,
        user_id: UserId,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            poll_id,
            option_id,
            user_id,
            created_at,
        }
    }

    pub fn poll_id(&self) -> PollId {
        self.poll_id
    }

    pub fn option_id(&self) -> PollOptionId {
        self.option_id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(n: u128, text: &str) -> PollOption {
        PollOption::new(
            PollOptionId::new(uuid::Uuid::from_u128(n)),
            Question::parse(text).unwrap(),
        )
    }

    fn poll_with(options: Vec<PollOption>) -> Result<Poll, PollError> {
        Poll::new(
            PollId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::nil()),
            Question::parse("Which one?").unwrap(),
            options,
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn parses_a_question() {
        assert_eq!(Question::parse("  Why?  ").unwrap().as_str(), "Why?");
        assert_eq!(Question::parse("   "), Err(QuestionError::Empty));
        assert_eq!(
            Question::parse(&"a".repeat(QUESTION_MAX_LEN + 1)),
            Err(QuestionError::TooLong)
        );
    }

    #[test]
    fn builds_a_poll_with_its_options() {
        let poll = poll_with(vec![option(1, "First"), option(2, "Second")]).unwrap();
        assert_eq!(poll.options().len(), 2);
        assert_eq!(poll.question().as_str(), "Which one?");
        assert!(poll.has_option(PollOptionId::new(uuid::Uuid::from_u128(1))));
        assert!(!poll.has_option(PollOptionId::new(uuid::Uuid::from_u128(9))));
    }

    #[test]
    fn rejects_fewer_than_two_options() {
        assert_eq!(poll_with(vec![]), Err(PollError::TooFewOptions));
        assert_eq!(
            poll_with(vec![option(1, "Only")]),
            Err(PollError::TooFewOptions)
        );
    }

    #[test]
    fn rejects_more_than_the_maximum_options() {
        let options = (0..=MAX_OPTIONS as u128)
            .map(|n| option(n, "Option"))
            .collect();
        assert_eq!(poll_with(options), Err(PollError::TooManyOptions));
    }

    #[test]
    fn a_vote_exposes_its_fields() {
        let vote = Vote::new(
            PollId::new(uuid::Uuid::nil()),
            PollOptionId::new(uuid::Uuid::from_u128(1)),
            UserId::new(uuid::Uuid::max()),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(vote.poll_id(), PollId::new(uuid::Uuid::nil()));
        assert_eq!(vote.option_id(), PollOptionId::new(uuid::Uuid::from_u128(1)));
        assert_eq!(vote.user_id(), UserId::new(uuid::Uuid::max()));
    }
}
