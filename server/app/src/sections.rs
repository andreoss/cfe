use crate::ports::{GroupRepository, SectionRepository};
use domain::{Group, PostScore, Section, SectionId, Slug, Title, User};

#[derive(Debug, PartialEq, Eq)]
pub enum SectionError {
    NotAuthorized,
    SlugTaken,
    NotFound,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GroupEditError {
    NotAuthorized,
    NotFound,
}

pub async fn create_section(
    sections: &(impl SectionRepository + ?Sized),
    operator: &User,
    id: SectionId,
    slug: Slug,
    title: Title,
) -> Result<Section, SectionError> {
    if !operator.role().is_moderator() {
        return Err(SectionError::NotAuthorized);
    }
    if sections.find_by_slug(&slug).await.is_some() {
        return Err(SectionError::SlugTaken);
    }
    let section = Section::new(id, slug, title);
    sections.save(&section).await;
    Ok(section)
}

pub async fn rename_section(
    sections: &(impl SectionRepository + ?Sized),
    operator: &User,
    slug: &Slug,
    title: Title,
) -> Result<Section, SectionError> {
    let section = editable(sections, operator, slug).await?;
    let renamed = Section::from_parts(
        section.id(),
        section.slug().clone(),
        title,
        section.topics_score(),
    );
    sections.update(&renamed).await;
    Ok(renamed)
}

pub async fn set_section_score(
    sections: &(impl SectionRepository + ?Sized),
    operator: &User,
    slug: &Slug,
    score: PostScore,
) -> Result<Section, SectionError> {
    let section = editable(sections, operator, slug).await?;
    let changed = section.with_topics_score(score);
    sections.update(&changed).await;
    Ok(changed)
}

async fn editable(
    sections: &(impl SectionRepository + ?Sized),
    operator: &User,
    slug: &Slug,
) -> Result<Section, SectionError> {
    if !operator.role().is_moderator() {
        return Err(SectionError::NotAuthorized);
    }
    sections
        .find_by_slug(slug)
        .await
        .ok_or(SectionError::NotFound)
}

pub async fn rename_group(
    sections: &(impl SectionRepository + ?Sized),
    groups: &(impl GroupRepository + ?Sized),
    operator: &User,
    section_slug: &Slug,
    group_slug: &Slug,
    name: Title,
) -> Result<Group, GroupEditError> {
    if !operator.role().is_moderator() {
        return Err(GroupEditError::NotAuthorized);
    }
    let section = sections
        .find_by_slug(section_slug)
        .await
        .ok_or(GroupEditError::NotFound)?;
    let group = groups
        .find_by_slug(section.id(), group_slug)
        .await
        .ok_or(GroupEditError::NotFound)?;
    let renamed = Group::from_parts(group.id(), group.section_id(), name, group.slug().clone());
    groups.update(&renamed).await;
    Ok(renamed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeGroupRepo, FakeSectionRepo};
    use domain::{Email, GroupId, UserId, Username};

    fn user(name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(1)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn operator() -> User {
        user("keeper_01").promoted_to_moderator()
    }

    fn slug(raw: &str) -> Slug {
        Slug::parse(raw).unwrap()
    }

    fn title(raw: &str) -> Title {
        Title::parse(raw).unwrap()
    }

    fn id(n: u128) -> SectionId {
        SectionId::new(uuid::Uuid::from_u128(n))
    }

    async fn with_section() -> FakeSectionRepo {
        let repo = FakeSectionRepo::new();
        create_section(&repo, &operator(), id(1), slug("general"), title("General"))
            .await
            .unwrap();
        repo
    }

    #[tokio::test]
    async fn an_operator_makes_a_section() {
        let repo = FakeSectionRepo::new();
        let section = create_section(&repo, &operator(), id(1), slug("news"), title("News"))
            .await
            .unwrap();
        assert_eq!(section.slug(), &slug("news"));
        assert_eq!(section.title(), &title("News"));
        assert!(repo.find_by_slug(&slug("news")).await.is_some());
    }

    #[tokio::test]
    async fn a_new_section_is_open_to_anyone_who_registered() {
        let repo = FakeSectionRepo::new();
        let section = create_section(&repo, &operator(), id(1), slug("news"), title("News"))
            .await
            .unwrap();
        assert_eq!(section.topics_score(), PostScore::default());
    }

    #[tokio::test]
    async fn nobody_else_may_make_one() {
        let repo = FakeSectionRepo::new();
        assert_eq!(
            create_section(
                &repo,
                &user("reader_01"),
                id(1),
                slug("news"),
                title("News")
            )
            .await,
            Err(SectionError::NotAuthorized)
        );
        assert!(repo.find_by_slug(&slug("news")).await.is_none());
    }

    #[tokio::test]
    async fn a_slug_is_not_taken_twice() {
        let repo = with_section().await;
        assert_eq!(
            create_section(&repo, &operator(), id(2), slug("general"), title("Another")).await,
            Err(SectionError::SlugTaken)
        );
    }

    #[tokio::test]
    async fn an_operator_renames_a_section() {
        let repo = with_section().await;
        let renamed = rename_section(&repo, &operator(), &slug("general"), title("General Talk"))
            .await
            .unwrap();
        assert_eq!(renamed.title(), &title("General Talk"));
        assert_eq!(
            repo.find_by_slug(&slug("general")).await.unwrap().title(),
            &title("General Talk")
        );
    }

    #[tokio::test]
    async fn renaming_leaves_the_slug_alone_so_links_still_work() {
        let repo = with_section().await;
        let renamed = rename_section(&repo, &operator(), &slug("general"), title("Renamed"))
            .await
            .unwrap();
        assert_eq!(renamed.slug(), &slug("general"));
    }

    #[tokio::test]
    async fn nobody_else_may_rename_one() {
        let repo = with_section().await;
        assert_eq!(
            rename_section(&repo, &user("reader_01"), &slug("general"), title("Mine")).await,
            Err(SectionError::NotAuthorized)
        );
    }

    #[tokio::test]
    async fn renaming_one_that_is_not_there_is_refused() {
        let repo = with_section().await;
        assert_eq!(
            rename_section(&repo, &operator(), &slug("missing"), title("Nothing")).await,
            Err(SectionError::NotFound)
        );
    }

    #[tokio::test]
    async fn an_operator_sets_what_it_takes_to_post_there() {
        let repo = with_section().await;
        let changed = set_section_score(
            &repo,
            &operator(),
            &slug("general"),
            PostScore::ModeratorsOnly,
        )
        .await
        .unwrap();
        assert_eq!(changed.topics_score(), PostScore::ModeratorsOnly);
        assert_eq!(
            repo.find_by_slug(&slug("general"))
                .await
                .unwrap()
                .topics_score(),
            PostScore::ModeratorsOnly
        );
    }

    #[tokio::test]
    async fn setting_the_standing_keeps_the_name() {
        let repo = with_section().await;
        let changed = set_section_score(
            &repo,
            &operator(),
            &slug("general"),
            PostScore::Floor(domain::FLOOR_100),
        )
        .await
        .unwrap();
        assert_eq!(changed.title(), &title("General"));
        assert_eq!(changed.topics_score(), PostScore::Floor(domain::FLOOR_100));
    }

    #[tokio::test]
    async fn nobody_else_may_set_it() {
        let repo = with_section().await;
        assert_eq!(
            set_section_score(
                &repo,
                &user("reader_01"),
                &slug("general"),
                PostScore::NoComments
            )
            .await,
            Err(SectionError::NotAuthorized)
        );
    }

    async fn with_group() -> (FakeSectionRepo, FakeGroupRepo) {
        let sections = with_section().await;
        let groups = FakeGroupRepo::new();
        let group = Group::new(
            GroupId::new(uuid::Uuid::from_u128(7)),
            id(1),
            title("Old Name"),
            slug("a-group"),
        );
        groups.save(&group).await;
        (sections, groups)
    }

    #[tokio::test]
    async fn an_operator_renames_a_group() {
        let (sections, groups) = with_group().await;
        let renamed = rename_group(
            &sections,
            &groups,
            &operator(),
            &slug("general"),
            &slug("a-group"),
            title("New Name"),
        )
        .await
        .unwrap();
        assert_eq!(renamed.name(), &title("New Name"));
        assert_eq!(renamed.slug(), &slug("a-group"));
        assert_eq!(
            groups
                .find_by_slug(id(1), &slug("a-group"))
                .await
                .unwrap()
                .name(),
            &title("New Name")
        );
    }

    #[tokio::test]
    async fn nobody_else_may_rename_a_group() {
        let (sections, groups) = with_group().await;
        assert_eq!(
            rename_group(
                &sections,
                &groups,
                &user("reader_01"),
                &slug("general"),
                &slug("a-group"),
                title("Mine")
            )
            .await,
            Err(GroupEditError::NotAuthorized)
        );
    }

    #[tokio::test]
    async fn renaming_a_group_that_is_not_there_is_refused() {
        let (sections, groups) = with_group().await;
        assert_eq!(
            rename_group(
                &sections,
                &groups,
                &operator(),
                &slug("general"),
                &slug("missing"),
                title("Nothing")
            )
            .await,
            Err(GroupEditError::NotFound)
        );
        assert_eq!(
            rename_group(
                &sections,
                &groups,
                &operator(),
                &slug("missing"),
                &slug("a-group"),
                title("Nothing")
            )
            .await,
            Err(GroupEditError::NotFound)
        );
    }
}
