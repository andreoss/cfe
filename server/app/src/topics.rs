use crate::ports::{SectionRepository, TopicRepository};
use crate::paging::Paged;
use domain::{Body, Page, Section, Slug, TagSet, Title, Topic, TopicId, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum CreateTopicError {
    SectionNotFound,
}

pub async fn create_topic(
    sections: &(impl SectionRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    id: TopicId,
    slug: &Slug,
    author_id: UserId,
    title: Title,
    body: Body,
    tags: TagSet,
    now: OffsetDateTime,
) -> Result<Topic, CreateTopicError> {
    let section = sections
        .find_by_slug(slug)
        .await
        .ok_or(CreateTopicError::SectionNotFound)?;
    let topic = Topic::new(id, section.id(), author_id, title, body, tags, now);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeSectionRepo, FakeTopicRepo};
    use domain::SectionId as DomainSectionId;

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
            UserId::new(uuid::Uuid::nil()),
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
            UserId::new(uuid::Uuid::nil()),
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
            UserId::new(uuid::Uuid::nil()),
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
            UserId::new(uuid::Uuid::nil()),
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
}
