use crate::ports::TopicRepository;
use domain::{Topic, TopicId, User};

#[derive(Debug, PartialEq, Eq)]
pub enum FlagError {
    NotFound,
    NotAuthorized,
}

async fn find(
    topics: &(impl TopicRepository + ?Sized),
    topic_id: TopicId,
) -> Result<Topic, FlagError> {
    topics.find_by_id(topic_id).await.ok_or(FlagError::NotFound)
}

pub async fn publish_draft(
    topics: &(impl TopicRepository + ?Sized),
    author: &User,
    topic_id: TopicId,
) -> Result<Topic, FlagError> {
    let topic = find(topics, topic_id).await?;
    if topic.author_id() != author.id() {
        return Err(FlagError::NotAuthorized);
    }
    let published = topic.with_draft(false);
    topics.update(&published).await;
    Ok(published)
}

pub async fn set_sticky(
    topics: &(impl TopicRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
    sticky: bool,
) -> Result<Topic, FlagError> {
    if !moderator.role().is_moderator() {
        return Err(FlagError::NotAuthorized);
    }
    let topic = find(topics, topic_id).await?;
    let updated = topic.with_sticky(sticky);
    topics.update(&updated).await;
    Ok(updated)
}

pub async fn set_off_front(
    topics: &(impl TopicRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
    off_front: bool,
) -> Result<Topic, FlagError> {
    if !moderator.role().is_moderator() {
        return Err(FlagError::NotAuthorized);
    }
    let topic = find(topics, topic_id).await?;
    let updated = topic.with_off_front(off_front);
    topics.update(&updated).await;
    Ok(updated)
}

pub async fn set_resolved(
    topics: &(impl TopicRepository + ?Sized),
    user: &User,
    topic_id: TopicId,
    resolved: bool,
) -> Result<Topic, FlagError> {
    let topic = find(topics, topic_id).await?;
    if topic.author_id() != user.id() && !user.role().is_moderator() {
        return Err(FlagError::NotAuthorized);
    }
    let updated = topic.with_resolved(resolved);
    topics.update(&updated).await;
    Ok(updated)
}

pub fn visible_to(topic: &Topic, viewer: Option<&User>) -> bool {
    if !topic.is_draft() {
        return true;
    }
    match viewer {
        Some(user) => topic.author_id() == user.id(),
        None => false,
    }
}

pub fn order_sticky_first(mut topics: Vec<Topic>) -> Vec<Topic> {
    topics.sort_by_key(|t| !t.is_sticky());
    topics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeTopicRepo;
    use domain::{Body, Email, SectionId, TagSet, Title, UserId, Username};
    use time::OffsetDateTime;

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn moderator() -> User {
        user(99, "keeper_01").promoted_to_moderator()
    }

    fn topic_by(author: &User, id: u128) -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::from_u128(id)),
            SectionId::new(uuid::Uuid::nil()),
            author.id(),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn an_author_publishes_their_own_draft() {
        let author = user(1, "author_01");
        let draft = topic_by(&author, 1).with_draft(true);
        let topics = FakeTopicRepo::with(draft.clone());
        let published = publish_draft(&topics, &author, draft.id()).await.unwrap();
        assert!(!published.is_draft());
        assert!(!topics.find_by_id(draft.id()).await.unwrap().is_draft());
    }

    #[tokio::test]
    async fn another_account_cannot_publish_someone_elses_draft() {
        let author = user(1, "author_01");
        let draft = topic_by(&author, 2).with_draft(true);
        let topics = FakeTopicRepo::with(draft.clone());
        let result = publish_draft(&topics, &user(2, "other_01"), draft.id()).await;
        assert_eq!(result, Err(FlagError::NotAuthorized));
    }

    #[tokio::test]
    async fn a_draft_is_visible_only_to_its_author() {
        let author = user(1, "author_01");
        let draft = topic_by(&author, 3).with_draft(true);
        assert!(visible_to(&draft, Some(&author)));
        assert!(!visible_to(&draft, Some(&user(2, "other_01"))));
        assert!(!visible_to(&draft, None));
    }

    #[tokio::test]
    async fn a_published_topic_is_visible_to_anyone() {
        let author = user(1, "author_01");
        let topic = topic_by(&author, 4);
        assert!(visible_to(&topic, None));
        assert!(visible_to(&topic, Some(&user(2, "other_01"))));
    }

    #[tokio::test]
    async fn only_a_moderator_makes_a_topic_sticky() {
        let author = user(1, "author_01");
        let topic = topic_by(&author, 5);
        let topics = FakeTopicRepo::with(topic.clone());
        assert_eq!(
            set_sticky(&topics, &author, topic.id(), true).await,
            Err(FlagError::NotAuthorized)
        );
        let stuck = set_sticky(&topics, &moderator(), topic.id(), true)
            .await
            .unwrap();
        assert!(stuck.is_sticky());
    }

    #[tokio::test]
    async fn only_a_moderator_keeps_a_topic_off_the_front() {
        let author = user(1, "author_01");
        let topic = topic_by(&author, 6);
        let topics = FakeTopicRepo::with(topic.clone());
        assert_eq!(
            set_off_front(&topics, &author, topic.id(), true).await,
            Err(FlagError::NotAuthorized)
        );
        let hidden = set_off_front(&topics, &moderator(), topic.id(), true)
            .await
            .unwrap();
        assert!(hidden.is_off_front());
    }

    #[tokio::test]
    async fn the_author_or_a_moderator_marks_a_thread_resolved() {
        let author = user(1, "author_01");
        let topic = topic_by(&author, 7);
        let topics = FakeTopicRepo::with(topic.clone());
        assert_eq!(
            set_resolved(&topics, &user(2, "other_01"), topic.id(), true).await,
            Err(FlagError::NotAuthorized)
        );
        assert!(
            set_resolved(&topics, &author, topic.id(), true)
                .await
                .unwrap()
                .is_resolved()
        );
        assert!(
            !set_resolved(&topics, &moderator(), topic.id(), false)
                .await
                .unwrap()
                .is_resolved()
        );
    }

    #[tokio::test]
    async fn flagging_an_unknown_topic_is_refused() {
        let topics = FakeTopicRepo::new();
        let missing = TopicId::new(uuid::Uuid::from_u128(404));
        assert_eq!(
            set_sticky(&topics, &moderator(), missing, true).await,
            Err(FlagError::NotFound)
        );
        assert_eq!(
            set_resolved(&topics, &moderator(), missing, true).await,
            Err(FlagError::NotFound)
        );
    }

    #[tokio::test]
    async fn sticky_topics_come_first_and_the_rest_keep_their_order() {
        let author = user(1, "author_01");
        let first = topic_by(&author, 10);
        let stuck = topic_by(&author, 11).with_sticky(true);
        let last = topic_by(&author, 12);
        let ordered = order_sticky_first(vec![first.clone(), stuck.clone(), last.clone()]);
        assert_eq!(ordered[0].id(), stuck.id());
        assert_eq!(ordered[1].id(), first.id());
        assert_eq!(ordered[2].id(), last.id());
    }
}
