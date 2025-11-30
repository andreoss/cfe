use crate::ports::{GroupRepository, SectionRepository, TopicRepository};
use crate::paging::Paged;
use domain::{Body, GroupId, Page, Section, Slug, TagSet, Title, Topic, TopicId, User, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility {
    viewer: Option<UserId>,
    is_moderator: bool,
}

impl Visibility {
    pub fn anonymous() -> Self {
        Self {
            viewer: None,
            is_moderator: false,
        }
    }

    pub fn moderator() -> Self {
        Self {
            viewer: None,
            is_moderator: true,
        }
    }

    pub fn of(viewer: Option<&User>) -> Self {
        Self {
            viewer: viewer.map(|u| u.id()),
            is_moderator: viewer.map(|u| u.role().is_moderator()).unwrap_or(false),
        }
    }

    pub fn allows(&self, topic: &Topic) -> bool {
        !topic.is_pending() || self.is_moderator || Some(topic.author_id()) == self.viewer
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CreateTopicError {
    SectionNotFound,
    Restricted,
}

pub fn may_start_topic(section: &domain::Section, author: &User) -> bool {
    section
        .topics_score()
        .allows(author.score(), author.role().is_moderator(), false)
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
    group_id: Option<GroupId>,
    now: OffsetDateTime,
) -> Result<Topic, CreateTopicError> {
    let section = sections
        .find_by_slug(slug)
        .await
        .ok_or(CreateTopicError::SectionNotFound)?;
    if !may_start_topic(&section, author) {
        return Err(CreateTopicError::Restricted);
    }
    let pending = !author.role().is_moderator();
    let topic = Topic::new(id, section.id(), author.id(), title, body, tags, now)
        .with_group(group_id)
        .with_pending(pending);
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
    visibility: Visibility,
) -> Result<Paged<Topic>, ListTopicsError> {
    let section = sections
        .find_by_slug(slug)
        .await
        .ok_or(ListTopicsError::SectionNotFound)?;
    let mut items = topics.list_by_section(section.id(), page).await;
    items.retain(|t| visibility.allows(t));
    let total = topics.count_by_section(section.id()).await;
    Ok(Paged::new(items, page, total))
}

pub async fn list_topics_by_tag(
    topics: &(impl TopicRepository + ?Sized),
    tag: &Slug,
    page: Page,
    visibility: Visibility,
) -> Paged<Topic> {
    let mut items = topics.list_by_tag(tag, page).await;
    items.retain(|t| visibility.allows(t));
    let total = topics.count_by_tag(tag).await;
    Paged::new(items, page, total)
}

pub async fn get_topic(
    topics: &(impl TopicRepository + ?Sized),
    id: TopicId,
    visibility: Visibility,
) -> Option<Topic> {
    let topic = topics.find_by_id(id).await?;
    if !visibility.allows(&topic) {
        return None;
    }
    Some(topic)
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

#[derive(Debug, PartialEq, Eq)]
pub enum CommitTopicError {
    NotFound,
    NotAuthorized,
}

pub async fn commit_topic(
    topics: &(impl TopicRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
) -> Result<Topic, CommitTopicError> {
    if !moderator.role().is_moderator() {
        return Err(CommitTopicError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(CommitTopicError::NotFound)?;
    let updated = topic.with_pending(false);
    topics.update(&updated).await;
    Ok(updated)
}

pub async fn uncommit_topic(
    topics: &(impl TopicRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
) -> Result<Topic, CommitTopicError> {
    if !moderator.role().is_moderator() {
        return Err(CommitTopicError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(CommitTopicError::NotFound)?;
    let updated = topic.with_pending(true);
    topics.update(&updated).await;
    Ok(updated)
}

#[derive(Debug, PartialEq, Eq)]
pub enum MoveTopicError {
    NotFound,
    NotAuthorized,
    GroupNotFound,
    WrongSection,
}

pub async fn move_topic(
    topics: &(impl TopicRepository + ?Sized),
    groups: &(impl GroupRepository + ?Sized),
    moderator: &User,
    topic_id: TopicId,
    group_id: GroupId,
) -> Result<Topic, MoveTopicError> {
    if !moderator.role().is_moderator() {
        return Err(MoveTopicError::NotAuthorized);
    }
    let topic = topics
        .find_by_id(topic_id)
        .await
        .ok_or(MoveTopicError::NotFound)?;
    let group = groups
        .find_by_id(group_id)
        .await
        .ok_or(MoveTopicError::GroupNotFound)?;
    if group.section_id() != topic.section_id() {
        return Err(MoveTopicError::WrongSection);
    }
    let updated = topic.with_group(Some(group_id));
    topics.update(&updated).await;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeGroupRepo, FakeSectionRepo, FakeTopicRepo};
    use domain::SectionId as DomainSectionId;
    use domain::{Group, GroupId, UserId};

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
            None,
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
            None,
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
            None,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let listed = list_topics(&sections, &topics, &Slug::parse("general").unwrap(), Page::first(), Visibility::moderator())
            .await
            .unwrap();
        assert_eq!(listed.items.len(), 1);
    }

    #[tokio::test]
    async fn rejects_listing_an_unknown_section() {
        let sections = FakeSectionRepo::new();
        let topics = FakeTopicRepo::new();
        let result = list_topics(&sections, &topics, &Slug::parse("ghost").unwrap(), Page::first(), Visibility::moderator()).await;
        assert!(matches!(result, Err(ListTopicsError::SectionNotFound)));
    }

    #[tokio::test]
    async fn get_topic_returns_none_for_unknown_id() {
        let topics = FakeTopicRepo::new();
        assert!(
            get_topic(&topics, TopicId::new(uuid::Uuid::nil()), Visibility::moderator())
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
            None,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let listed = list_topics_by_tag(&topics, &Slug::parse("rust").unwrap(), Page::first(), Visibility::moderator()).await;
        assert_eq!(listed.items.len(), 1);
        let empty = list_topics_by_tag(&topics, &Slug::parse("nothing").unwrap(), Page::first(), Visibility::moderator()).await;
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
            None,
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
            None,
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
            None,
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
        let stored = get_topic(&topics, TopicId::new(uuid::Uuid::nil()), Visibility::moderator()).await.unwrap();
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

    #[tokio::test]
    async fn a_topic_from_a_plain_user_starts_pending() {
        let sections = FakeSectionRepo::with(section());
        let topics = FakeTopicRepo::new();
        let topic = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            None,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(topic.is_pending());
        let listed = list_topics(
            &sections,
            &topics,
            &Slug::parse("general").unwrap(),
            Page::first(),
            Visibility::anonymous(),
        )
        .await
        .unwrap();
        assert_eq!(listed.items.len(), 0);
        let visible = list_topics(
            &sections,
            &topics,
            &Slug::parse("general").unwrap(),
            Page::first(),
            Visibility::moderator(),
        )
        .await
        .unwrap();
        assert_eq!(visible.items.len(), 1);
    }

    #[tokio::test]
    async fn an_author_sees_their_own_queued_topic_but_a_stranger_does_not() {
        let sections = FakeSectionRepo::with(section());
        let topics = FakeTopicRepo::new();
        let author = plain_user(uuid::Uuid::nil());
        let stranger = plain_user(uuid::Uuid::max());
        let topic = create_topic(
            &sections,
            &topics,
            TopicId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            &author,
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            tags(&[]),
            None,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(topic.is_pending());

        let mine = list_topics(
            &sections,
            &topics,
            &Slug::parse("general").unwrap(),
            Page::first(),
            Visibility::of(Some(&author)),
        )
        .await
        .unwrap();
        assert_eq!(mine.items.len(), 1);

        let theirs = list_topics(
            &sections,
            &topics,
            &Slug::parse("general").unwrap(),
            Page::first(),
            Visibility::of(Some(&stranger)),
        )
        .await
        .unwrap();
        assert_eq!(theirs.items.len(), 0);

        assert!(get_topic(&topics, topic.id(), Visibility::of(Some(&author))).await.is_some());
        assert!(get_topic(&topics, topic.id(), Visibility::of(Some(&stranger))).await.is_none());
        assert!(get_topic(&topics, topic.id(), Visibility::anonymous()).await.is_none());
    }

    #[tokio::test]
    async fn a_moderator_topic_starts_committed() {
        let sections = FakeSectionRepo::with(section());
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
            None,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(topic.is_committed());
    }

    #[tokio::test]
    async fn a_moderator_commits_and_uncommits_a_topic() {
        let topics = FakeTopicRepo::with(
            Topic::new(
                TopicId::new(uuid::Uuid::nil()),
                DomainSectionId::new(uuid::Uuid::nil()),
                UserId::new(uuid::Uuid::nil()),
                Title::parse("Hello").unwrap(),
                Body::parse("World").unwrap(),
                TagSet::empty(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .with_pending(true),
        );
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let committed = commit_topic(&topics, &moderator, TopicId::new(uuid::Uuid::nil()))
            .await
            .unwrap();
        assert!(committed.is_committed());
        let uncommitted = uncommit_topic(&topics, &moderator, TopicId::new(uuid::Uuid::nil()))
            .await
            .unwrap();
        assert!(uncommitted.is_pending());
        let refused = uncommit_topic(&topics, &plain_user(uuid::Uuid::nil()), TopicId::new(uuid::Uuid::nil())).await;
        assert_eq!(refused, Err(CommitTopicError::NotAuthorized));
    }

    #[tokio::test]
    async fn a_moderator_moves_a_topic_between_groups_in_one_section() {
        let topics = FakeTopicRepo::with(
            Topic::new(
                TopicId::new(uuid::Uuid::nil()),
                DomainSectionId::new(uuid::Uuid::nil()),
                UserId::new(uuid::Uuid::nil()),
                Title::parse("Hello").unwrap(),
                Body::parse("World").unwrap(),
                TagSet::empty(),
                OffsetDateTime::UNIX_EPOCH,
            ),
        );
        let groups = FakeGroupRepo::new();
        let group = Group::new(
            GroupId::new(uuid::Uuid::max()),
            DomainSectionId::new(uuid::Uuid::nil()),
            Title::parse("Announcements").unwrap(),
            Slug::parse("announcements").unwrap(),
        );
        groups.save(&group).await;
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let moved = move_topic(
            &topics,
            &groups,
            &moderator,
            TopicId::new(uuid::Uuid::nil()),
            GroupId::new(uuid::Uuid::max()),
        )
        .await
        .unwrap();
        assert_eq!(moved.group_id(), Some(GroupId::new(uuid::Uuid::max())));
        let wrong = move_topic(
            &topics,
            &groups,
            &moderator,
            TopicId::new(uuid::Uuid::nil()),
            GroupId::new(uuid::Uuid::nil()),
        )
        .await;
        assert_eq!(wrong, Err(MoveTopicError::GroupNotFound));
    }

    #[tokio::test]
    async fn a_move_rejects_a_group_in_another_section() {
        let topics = FakeTopicRepo::with(
            Topic::new(
                TopicId::new(uuid::Uuid::nil()),
                DomainSectionId::new(uuid::Uuid::nil()),
                UserId::new(uuid::Uuid::nil()),
                Title::parse("Hello").unwrap(),
                Body::parse("World").unwrap(),
                TagSet::empty(),
                OffsetDateTime::UNIX_EPOCH,
            ),
        );
        let groups = FakeGroupRepo::new();
        let other = Group::new(
            GroupId::new(uuid::Uuid::max()),
            DomainSectionId::new(uuid::Uuid::max()),
            Title::parse("Elsewhere").unwrap(),
            Slug::parse("elsewhere").unwrap(),
        );
        groups.save(&other).await;
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let result = move_topic(
            &topics,
            &groups,
            &moderator,
            TopicId::new(uuid::Uuid::nil()),
            GroupId::new(uuid::Uuid::max()),
        )
        .await;
        assert_eq!(result, Err(MoveTopicError::WrongSection));
    }
}
