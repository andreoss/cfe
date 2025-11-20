use crate::ports::{SectionRepository, TopicRepository};
use crate::paging::Paged;
use domain::{Body, Page, Section, Slug, TagSet, Title, Topic, TopicId, User};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum CreateTopicError {
    SectionNotFound,
    Restricted,
}

pub async fn create_topic(
    sections: &(impl SectionRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    id: TopicId,
    slug: &Slug,
    author: &User,
    title: Title,
    body: Body,
    tags: TagSet,
    now: OffsetDateTime,
) -> Result<Topic, CreateTopicError> {
    let section = sections
        .find_by_slug(slug)
        .await
        .ok_or(CreateTopicError::SectionNotFound)?;
    let allowed = section
        .topics_score()
        .allows(author.score(), author.role().is_moderator(), false);
    if !allowed {
        return Err(CreateTopicError::Restricted);
    }
    let topic = Topic::new(id, section.id(), author.id(), title, body, tags, now);
    topics.save(&topic).await;
    Ok(topic)
}

pub async fn list_sections(sections: &(impl SectionRepository + ?Sized)) -> Vec<Section> {
    sections.list().await
}

#[derive(Debug, PartialEq, Eq)]
pub enum ListTopicsError {
    SectionNotFound,
}

pub async fn list_topics(
    sections: &(impl SectionRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    slug: &Slug,
    page: Page,
) -> Result<Paged<Topic>, ListTopicsError> {
    let section = sections
        .find_by_slug(slug)
        .await
        .ok_or(ListTopicsError::SectionNotFound)?;
    let items = topics.list_by_section(section.id(), page).await;
    let total = topics.count_by_section(section.id()).await;
    Ok(Paged::new(items, page, total))
}

pub async fn list_topics_by_tag(
    topics: &(impl TopicRepository + ?Sized),
    tag: &Slug,
    page: Page,
) -> Paged<Topic> {
    let items = topics.list_by_tag(tag, page).await;
    let total = topics.count_by_tag(tag).await;
    Paged::new(items, page, total)
}

pub async fn get_topic(topics: &(impl TopicRepository + ?Sized), id: TopicId) -> Option<Topic> {
    topics.find_by_id(id).await
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetPostscoreError {
    NotFound,
    NotAuthorized,
}

pub async fn set_postscore(
    topics: &(impl TopicRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
    postscore: domain::PostScore,
) -> Result<Topic, SetPostscoreError> {
    if !moderator.role().is_moderator() {
        return Err(SetPostscoreError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(SetPostscoreError::NotFound)?;
    let updated = topic.with_postscore(postscore);
    topics.update(&updated).await;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeSectionRepo, FakeTopicRepo};
    use domain::SectionId as DomainSectionId;
    use domain::UserId;

    fn section() -> Section {
        Section::new(
            DomainSectionId::new(uuid::Uuid::nil()),
            Slug::parse("general").unwrap(),
            Title::parse("General").unwrap(),
        )
    }

    fn tags(values: &[&str]) -> TagSet {
        TagSet::parse(&values.iter().map(|v| v.to_string()).collect::<Vec<_>>()).unwrap()
    }

    fn plain_user(id: uuid::Uuid) -> User {
        User::register(
            UserId::new(id),
            domain::Username::parse("alice_01").unwrap(),
            domain::Email::parse("alice@example.com").unwrap(),
            "hash".to_owned(),
        )
    }

    #[tokio::test]
    async fn creates_a_topic_in_an_existing_section() {
        let sections = FakeSectionRepo::with(section());
        let topics = FakeTopicRepo::new();
        let now = OffsetDateTime::UNIX_EPOCH;
        let topic = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            tags(&["rust"]),
            now,
        )
        .await
        .unwrap();
        assert_eq!(topic.section_id(), section().id());
        assert_eq!(topics.list_by_section(section().id(), Page::first()).await.len(), 1);
    }

    #[tokio::test]
    async fn rejects_creation_in_an_unknown_section() {
        let sections = FakeSectionRepo::new();
        let topics = FakeTopicRepo::new();
        let result = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("ghost").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(CreateTopicError::SectionNotFound));
    }

    #[tokio::test]
    async fn lists_topics_in_a_section() {
        let sections = FakeSectionRepo::with(section());
        let topics = FakeTopicRepo::new();
        create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let listed = list_topics(&sections, &topics, &Slug::parse("general").unwrap(), Page::first())
            .await
            .unwrap();
        assert_eq!(listed.items.len(), 1);
    }

    #[tokio::test]
    async fn rejects_listing_an_unknown_section() {
        let sections = FakeSectionRepo::new();
        let topics = FakeTopicRepo::new();
        let result = list_topics(&sections, &topics, &Slug::parse("ghost").unwrap(), Page::first()).await;
        assert!(matches!(result, Err(ListTopicsError::SectionNotFound)));
    }

    #[tokio::test]
    async fn get_topic_returns_none_for_unknown_id() {
        let topics = FakeTopicRepo::new();
        assert!(
            get_topic(&topics, TopicId::new(uuid::Uuid::nil()))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn lists_topics_by_tag() {
        let sections = FakeSectionRepo::with(section());
        let topics = FakeTopicRepo::new();
        create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            tags(&["rust"]),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let listed = list_topics_by_tag(&topics, &Slug::parse("rust").unwrap(), Page::first()).await;
        assert_eq!(listed.items.len(), 1);
        let empty = list_topics_by_tag(&topics, &Slug::parse("nothing").unwrap(), Page::first()).await;
        assert_eq!(empty.items.len(), 0);
    }

    #[tokio::test]
    async fn a_low_score_user_is_refused_in_a_restricted_section() {
        let mut section = section();
        section = section.with_topics_score(domain::PostScore::Floor(domain::FLOOR_50));
        let sections = FakeSectionRepo::with(section);
        let topics = FakeTopicRepo::new();
        let result = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(CreateTopicError::Restricted));
    }

    #[tokio::test]
    async fn a_moderator_can_post_in_a_restricted_section() {
        let mut section = section();
        section = section.with_topics_score(domain::PostScore::Floor(domain::FLOOR_500));
        let sections = FakeSectionRepo::with(section);
        let topics = FakeTopicRepo::new();
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let topic = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &moderator,
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(topic.author_id(), moderator.id());
    }

    #[tokio::test]
    async fn a_high_score_user_can_post_in_a_restricted_section() {
        let mut section = section();
        section = section.with_topics_score(domain::PostScore::Floor(domain::FLOOR_50));
        let sections = FakeSectionRepo::with(section);
        let topics = FakeTopicRepo::new();
        let qualified = plain_user(uuid::Uuid::nil()).with_score(domain::Score::of(50));
        let topic = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &qualified,
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(topic.author_id(), qualified.id());
    }

    #[tokio::test]
    async fn a_moderator_can_set_a_topics_comment_restriction() {
        let topics = FakeTopicRepo::with(
            Topic::new(
                TopicId::new(uuid::Uuid::nil()),
                DomainSectionId::new(uuid::Uuid::max()),
                UserId::new(uuid::Uuid::nil()),
                Title::parse("Hello").unwrap(),
                Body::parse("World").unwrap(),
                TagSet::empty(),
                OffsetDateTime::UNIX_EPOCH,
            ),
        );
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let updated = set_postscore(
            &topics,
            &moderator,
            TopicId::new(uuid::Uuid::nil()),
            domain::PostScore::NoComments,
        )
        .await
        .unwrap();
        assert_eq!(updated.postscore(), domain::PostScore::NoComments);
        let stored = get_topic(&topics, TopicId::new(uuid::Uuid::nil())).await.unwrap();
        assert_eq!(stored.postscore(), domain::PostScore::NoComments);
    }

    #[tokio::test]
    async fn a_plain_user_cannot_set_a_topics_restriction() {
        let topics = FakeTopicRepo::with(
            Topic::new(
                TopicId::new(uuid::Uuid::nil()),
                DomainSectionId::new(uuid::Uuid::max()),
                UserId::new(uuid::Uuid::nil()),
                Title::parse("Hello").unwrap(),
                Body::parse("World").unwrap(),
                TagSet::empty(),
                OffsetDateTime::UNIX_EPOCH,
            ),
        );
        let result = set_postscore(
            &topics,
            &plain_user(uuid::Uuid::max()),
            TopicId::new(uuid::Uuid::nil()),
            domain::PostScore::NoComments,
        )
        .await;
        assert_eq!(result, Err(SetPostscoreError::NotAuthorized));
    }

    #[tokio::test]
    async fn setting_a_restriction_on_an_unknown_topic_fails() {
        let topics = FakeTopicRepo::new();
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let result = set_postscore(
            &topics,
            &moderator,
            TopicId::new(uuid::Uuid::nil()),
            domain::PostScore::ModeratorsOnly,
        )
        .await;
        assert_eq!(result, Err(SetPostscoreError::NotFound));
    }
}
