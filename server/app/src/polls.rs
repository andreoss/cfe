use crate::ports::{PollRepository, TopicRepository};
use domain::{Poll, PollError, PollId, PollOption, PollOptionId, Question, TopicId, User, Vote};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum CreatePollError {
    TopicNotFound,
    NotTheAuthor,
    AlreadyExists,
    Invalid(PollError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum VoteError {
    PollNotFound,
    UnknownOption,
}

pub struct PollResults {
    pub poll: Poll,
    pub counts: Vec<(PollOptionId, u64)>,
    pub mine: Option<PollOptionId>,
}

pub async fn create_poll(
    topics: &(impl TopicRepository + ?Sized),
    polls: &(impl PollRepository + ?Sized),
    author: &User,
    id: PollId,
    topic_id: TopicId,
    question: Question,
    options: Vec<PollOption>,
    now: OffsetDateTime,
) -> Result<Poll, CreatePollError> {
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(CreatePollError::TopicNotFound)?;
    if topic.author_id() != author.id() {
        return Err(CreatePollError::NotTheAuthor);
    }
    if polls.find_by_topic(topic_id).await.is_some() {
        return Err(CreatePollError::AlreadyExists);
    }
    let poll = Poll::new(id, topic_id, question, options, now).map_err(CreatePollError::Invalid)?;
    polls.save(&poll).await;
    Ok(poll)
}

pub async fn cast_vote(
    polls: &(impl PollRepository + ?Sized),
    voter: &User,
    topic_id: TopicId,
    option_id: PollOptionId,
    now: OffsetDateTime,
) -> Result<(), VoteError> {
    let poll = polls
        .find_by_topic(topic_id)
        .await
        .ok_or(VoteError::PollNotFound)?;
    if !poll.has_option(option_id) {
        return Err(VoteError::UnknownOption);
    }
    polls
        .save_vote(&Vote::new(poll.id(), option_id, voter.id(), now))
        .await;
    Ok(())
}

pub async fn poll_results(
    polls: &(impl PollRepository + ?Sized),
    viewer_id: Option<domain::UserId>,
    topic_id: TopicId,
) -> Option<PollResults> {
    let poll = polls.find_by_topic(topic_id).await?;
    let counts = polls.counts(poll.id()).await;
    let mine = match viewer_id {
        Some(id) => polls.find_vote(poll.id(), id).await,
        None => None,
    };
    Some(PollResults { poll, counts, mine })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakePollRepo, FakeTopicRepo};
    use domain::{Body, Email, SectionId, TagSet, Title, Topic, UserId, Username};

    fn author_id() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn author() -> User {
        User::register(
            author_id(),
            Username::parse("poll_author").unwrap(),
            Email::parse("pa@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn stranger() -> User {
        User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("poll_other").unwrap(),
            Email::parse("po@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            author_id(),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn option(n: u128) -> PollOption {
        PollOption::new(
            PollOptionId::new(uuid::Uuid::from_u128(n)),
            Question::parse(&format!("Option {n}")).unwrap(),
        )
    }

    fn options() -> Vec<PollOption> {
        vec![option(1), option(2)]
    }

    async fn make_poll(
        topics: &FakeTopicRepo,
        polls: &FakePollRepo,
        who: &User,
    ) -> Result<Poll, CreatePollError> {
        create_poll(
            topics,
            polls,
            who,
            PollId::new(uuid::Uuid::nil()),
            topic().id(),
            Question::parse("Which one?").unwrap(),
            options(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
    }

    fn count_for(results: &PollResults, n: u128) -> u64 {
        let id = PollOptionId::new(uuid::Uuid::from_u128(n));
        results
            .counts
            .iter()
            .find(|(o, _)| *o == id)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    #[tokio::test]
    async fn the_topic_author_creates_a_poll() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        let poll = make_poll(&topics, &polls, &author()).await.unwrap();
        assert_eq!(poll.options().len(), 2);
        assert_eq!(poll.topic_id(), topic().id());
    }

    #[tokio::test]
    async fn a_stranger_cannot_add_a_poll_to_someone_elses_topic() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        let result = make_poll(&topics, &polls, &stranger()).await;
        assert_eq!(result, Err(CreatePollError::NotTheAuthor));
    }

    #[tokio::test]
    async fn a_topic_holds_at_most_one_poll() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        make_poll(&topics, &polls, &author()).await.unwrap();
        let result = make_poll(&topics, &polls, &author()).await;
        assert_eq!(result, Err(CreatePollError::AlreadyExists));
    }

    #[tokio::test]
    async fn rejects_a_poll_on_an_unknown_topic() {
        let topics = FakeTopicRepo::new();
        let polls = FakePollRepo::new();
        let result = make_poll(&topics, &polls, &author()).await;
        assert_eq!(result, Err(CreatePollError::TopicNotFound));
    }

    #[tokio::test]
    async fn rejects_a_poll_with_one_option() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        let result = create_poll(
            &topics,
            &polls,
            &author(),
            PollId::new(uuid::Uuid::nil()),
            topic().id(),
            Question::parse("Which one?").unwrap(),
            vec![option(1)],
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(
            result,
            Err(CreatePollError::Invalid(PollError::TooFewOptions))
        );
    }

    #[tokio::test]
    async fn a_vote_is_counted_and_reported_back_to_the_voter() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        make_poll(&topics, &polls, &author()).await.unwrap();
        cast_vote(
            &polls,
            &author(),
            topic().id(),
            option(1).id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let results = poll_results(&polls, Some(author_id()), topic().id())
            .await
            .unwrap();
        assert_eq!(count_for(&results, 1), 1);
        assert_eq!(results.mine, Some(option(1).id()));
    }

    #[tokio::test]
    async fn changing_your_vote_moves_it_rather_than_adding_one() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        make_poll(&topics, &polls, &author()).await.unwrap();
        for n in [1u128, 2] {
            cast_vote(
                &polls,
                &author(),
                topic().id(),
                option(n).id(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        let results = poll_results(&polls, Some(author_id()), topic().id())
            .await
            .unwrap();
        assert_eq!(count_for(&results, 1), 0);
        assert_eq!(count_for(&results, 2), 1);
        assert_eq!(results.mine, Some(option(2).id()));
    }

    #[tokio::test]
    async fn two_voters_each_count_once() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        make_poll(&topics, &polls, &author()).await.unwrap();
        for who in [author(), stranger()] {
            cast_vote(
                &polls,
                &who,
                topic().id(),
                option(1).id(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        let results = poll_results(&polls, None, topic().id()).await.unwrap();
        assert_eq!(count_for(&results, 1), 2);
        assert_eq!(results.mine, None);
    }

    #[tokio::test]
    async fn rejects_a_vote_for_an_option_from_another_poll() {
        let topics = FakeTopicRepo::with(topic());
        let polls = FakePollRepo::new();
        make_poll(&topics, &polls, &author()).await.unwrap();
        let result = cast_vote(
            &polls,
            &author(),
            topic().id(),
            PollOptionId::new(uuid::Uuid::from_u128(99)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(VoteError::UnknownOption));
    }

    #[tokio::test]
    async fn rejects_a_vote_when_the_topic_has_no_poll() {
        let polls = FakePollRepo::new();
        let result = cast_vote(
            &polls,
            &author(),
            topic().id(),
            option(1).id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(VoteError::PollNotFound));
        assert!(poll_results(&polls, None, topic().id()).await.is_none());
    }
}
