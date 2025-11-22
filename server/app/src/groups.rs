use crate::ports::{GroupRepository, SectionRepository};
use domain::{Group, GroupId, SectionId, Slug, Title, User};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum CreateGroupError {
    SectionNotFound,
    NotAuthorized,
    SlugTaken,
}

pub async fn create_group(
    sections: &(impl SectionRepository + ?Sized),
    groups: &(impl GroupRepository + ?Sized),
    id: GroupId,
    section_slug: &Slug,
    name: Title,
    slug: Slug,
    moderator: &User,
    now: OffsetDateTime,
) -> Result<Group, CreateGroupError> {
    if !moderator.role().is_moderator() {
        return Err(CreateGroupError::NotAuthorized);
    }
    let section = sections
        .find_by_slug(section_slug)
        .await
        .ok_or(CreateGroupError::SectionNotFound)?;
    if groups
        .find_by_slug(section.id(), &slug)
        .await
        .is_some()
    {
        return Err(CreateGroupError::SlugTaken);
    }
    let group = Group::new(id, section.id(), name, slug);
    groups.save(&group).await;
    Ok(group)
}

pub async fn list_groups(
    groups: &(impl GroupRepository + ?Sized),
    section_id: SectionId,
) -> Vec<Group> {
    groups.list_by_section(section_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeGroupRepo, FakeSectionRepo};
    use domain::{Section, UserId};

    fn section() -> Section {
        Section::new(
            SectionId::new(uuid::Uuid::nil()),
            Slug::parse("general").unwrap(),
            Title::parse("General").unwrap(),
        )
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
    async fn a_moderator_creates_a_group_in_a_section() {
        let sections = FakeSectionRepo::with(section());
        let groups = FakeGroupRepo::new();
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let group = create_group(
            &sections,
            &groups,
            GroupId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            Title::parse("Announcements").unwrap(),
            Slug::parse("announcements").unwrap(),
            &moderator,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(group.section_id(), section().id());
        assert_eq!(groups.list_by_section(section().id()).await.len(), 1);
    }

    #[tokio::test]
    async fn a_plain_user_cannot_create_a_group() {
        let sections = FakeSectionRepo::with(section());
        let groups = FakeGroupRepo::new();
        let result = create_group(
            &sections,
            &groups,
            GroupId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            Title::parse("Announcements").unwrap(),
            Slug::parse("announcements").unwrap(),
            &plain_user(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(CreateGroupError::NotAuthorized));
    }

    #[tokio::test]
    async fn a_duplicate_slug_is_refused() {
        let sections = FakeSectionRepo::with(section());
        let groups = FakeGroupRepo::new();
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let slug = Slug::parse("announcements").unwrap();
        create_group(
            &sections,
            &groups,
            GroupId::new(uuid::Uuid::nil()),
            &Slug::parse("general").unwrap(),
            Title::parse("Announcements").unwrap(),
            slug.clone(),
            &moderator,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let again = create_group(
            &sections,
            &groups,
            GroupId::new(uuid::Uuid::max()),
            &Slug::parse("general").unwrap(),
            Title::parse("Also").unwrap(),
            slug,
            &moderator,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(again, Err(CreateGroupError::SlugTaken));
    }

    #[tokio::test]
    async fn an_unknown_section_is_refused() {
        let sections = FakeSectionRepo::new();
        let groups = FakeGroupRepo::new();
        let moderator = plain_user(uuid::Uuid::max()).promoted_to_moderator();
        let result = create_group(
            &sections,
            &groups,
            GroupId::new(uuid::Uuid::nil()),
            &Slug::parse("ghost").unwrap(),
            Title::parse("Announcements").unwrap(),
            Slug::parse("announcements").unwrap(),
            &moderator,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(CreateGroupError::SectionNotFound));
    }
}
