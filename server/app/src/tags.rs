use crate::paging::Paged;
use crate::ports::{TagRepository, TopicRepository};
use crate::topics::Visibility;
use domain::{Page, Slug, Tag, TagDescription, Topic, User, UserId};

#[derive(Debug, PartialEq, Eq)]
pub enum TagError {
    NotAuthorized,
    MeansItself,
}

pub async fn describe_tag(
    tags: &(impl TagRepository + ?Sized),
    operator: &User,
    slug: &Slug,
    description: TagDescription,
) -> Result<Tag, TagError> {
    if !operator.role().is_moderator() {
        return Err(TagError::NotAuthorized);
    }
    let existing = tags
        .find(slug)
        .await
        .unwrap_or_else(|| Tag::new(slug.clone()));
    let described = existing.described(description);
    tags.save(&described).await;
    Ok(described)
}

pub async fn make_synonym(
    tags: &(impl TagRepository + ?Sized),
    operator: &User,
    slug: &Slug,
    means: Slug,
) -> Result<Tag, TagError> {
    if !operator.role().is_moderator() {
        return Err(TagError::NotAuthorized);
    }
    let existing = tags
        .find(slug)
        .await
        .unwrap_or_else(|| Tag::new(slug.clone()));
    let synonym = existing.meaning(means).ok_or(TagError::MeansItself)?;
    tags.save(&synonym).await;
    Ok(synonym)
}

pub async fn what_it_means(tags: &(impl TagRepository + ?Sized), slug: &Slug) -> Slug {
    let mut seen: Vec<Slug> = vec![slug.clone()];
    let mut current = slug.clone();
    for _ in 0..8 {
        let Some(tag) = tags.find(&current).await else {
            return current;
        };
        let Some(next) = tag.means() else {
            return current;
        };
        if seen.contains(next) {
            return current;
        }
        seen.push(next.clone());
        current = next.clone();
    }
    current
}

pub async fn topics_for_tag(
    tags: &(impl TagRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    slug: &Slug,
    page: Page,
    visibility: Visibility,
) -> Paged<Topic> {
    let canonical = what_it_means(tags, slug).await;
    let mut items = topics.list_by_tag(&canonical, page).await;
    items.retain(|t| visibility.allows(t));
    let total = topics.count_by_tag(&canonical).await;
    Paged::new(items, page, total)
}

pub async fn follow_tag(tags: &(impl TagRepository + ?Sized), follower: &User, slug: &Slug) {
    let canonical = what_it_means(tags, slug).await;
    tags.follow(follower.id(), &canonical).await;
}

pub async fn stop_following(tags: &(impl TagRepository + ?Sized), follower: &User, slug: &Slug) {
    let canonical = what_it_means(tags, slug).await;
    tags.unfollow(follower.id(), &canonical).await;
}

pub async fn is_following(
    tags: &(impl TagRepository + ?Sized),
    follower: &User,
    slug: &Slug,
) -> bool {
    let canonical = what_it_means(tags, slug).await;
    tags.is_following(follower.id(), &canonical).await
}

pub async fn followed_tags(tags: &(impl TagRepository + ?Sized), follower: &User) -> Vec<Slug> {
    tags.followed_by(follower.id()).await
}

pub async fn followers_of(tags: &(impl TagRepository + ?Sized), slug: &Slug) -> Vec<UserId> {
    tags.followers(slug).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeTagRepo, FakeTopicRepo};
    use domain::{Body, Email, SectionId, TagSet, Title, TopicId, Username};
    use time::OffsetDateTime;

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

    fn described(raw: &str) -> TagDescription {
        TagDescription::parse(raw).unwrap()
    }

    fn topic(n: u128, tags: &[&str]) -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::from_u128(n)),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::from_u128(9)),
            Title::parse(&format!("Subject {n}")).unwrap(),
            Body::parse("A body.").unwrap(),
            TagSet::parse(&tags.iter().map(|t| t.to_string()).collect::<Vec<_>>()).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .with_pending(false)
    }

    #[tokio::test]
    async fn an_operator_says_what_a_tag_is_for() {
        let repo = FakeTagRepo::new();
        let tag = describe_tag(&repo, &operator(), &slug("rust"), described("the language"))
            .await
            .unwrap();
        assert_eq!(tag.description().map(|d| d.as_str()), Some("the language"));
        assert!(repo.find(&slug("rust")).await.is_some());
    }

    #[tokio::test]
    async fn nobody_else_may_describe_one() {
        let repo = FakeTagRepo::new();
        assert_eq!(
            describe_tag(&repo, &user("reader_01"), &slug("rust"), described("mine")).await,
            Err(TagError::NotAuthorized)
        );
    }

    #[tokio::test]
    async fn one_tag_may_be_made_to_mean_another() {
        let repo = FakeTagRepo::new();
        make_synonym(&repo, &operator(), &slug("rustlang"), slug("rust"))
            .await
            .unwrap();
        assert_eq!(what_it_means(&repo, &slug("rustlang")).await, slug("rust"));
    }

    #[tokio::test]
    async fn a_tag_that_means_nothing_else_means_itself() {
        let repo = FakeTagRepo::new();
        assert_eq!(what_it_means(&repo, &slug("rust")).await, slug("rust"));
    }

    #[tokio::test]
    async fn a_tag_may_not_be_made_to_mean_itself() {
        let repo = FakeTagRepo::new();
        assert_eq!(
            make_synonym(&repo, &operator(), &slug("rust"), slug("rust")).await,
            Err(TagError::MeansItself)
        );
    }

    #[tokio::test]
    async fn a_chain_of_meanings_is_followed_to_the_end() {
        let repo = FakeTagRepo::new();
        make_synonym(&repo, &operator(), &slug("rs"), slug("rustlang"))
            .await
            .unwrap();
        make_synonym(&repo, &operator(), &slug("rustlang"), slug("rust"))
            .await
            .unwrap();
        assert_eq!(what_it_means(&repo, &slug("rs")).await, slug("rust"));
    }

    #[tokio::test]
    async fn a_circle_of_meanings_does_not_spin_forever() {
        let repo = FakeTagRepo::new();
        make_synonym(&repo, &operator(), &slug("one"), slug("two"))
            .await
            .unwrap();
        make_synonym(&repo, &operator(), &slug("two"), slug("one"))
            .await
            .unwrap();
        let landed = what_it_means(&repo, &slug("one")).await;
        assert!(landed == slug("one") || landed == slug("two"));
    }

    #[tokio::test]
    async fn a_synonym_shows_what_the_tag_it_means_holds() {
        let tags = FakeTagRepo::new();
        let topics = FakeTopicRepo::new();
        topics.save(&topic(1, &["rust"])).await;
        make_synonym(&tags, &operator(), &slug("rustlang"), slug("rust"))
            .await
            .unwrap();
        let found = topics_for_tag(
            &tags,
            &topics,
            &slug("rustlang"),
            Page::first(),
            Visibility::moderator(),
        )
        .await;
        assert_eq!(found.total, 1);
        assert_eq!(found.items.len(), 1);
    }

    #[tokio::test]
    async fn following_a_tag_and_letting_it_go() {
        let repo = FakeTagRepo::new();
        let reader = user("reader_01");
        assert!(!is_following(&repo, &reader, &slug("rust")).await);
        follow_tag(&repo, &reader, &slug("rust")).await;
        assert!(is_following(&repo, &reader, &slug("rust")).await);
        assert_eq!(followed_tags(&repo, &reader).await, vec![slug("rust")]);
        stop_following(&repo, &reader, &slug("rust")).await;
        assert!(!is_following(&repo, &reader, &slug("rust")).await);
    }

    #[tokio::test]
    async fn following_a_synonym_follows_what_it_means() {
        let repo = FakeTagRepo::new();
        let reader = user("reader_01");
        make_synonym(&repo, &operator(), &slug("rustlang"), slug("rust"))
            .await
            .unwrap();
        follow_tag(&repo, &reader, &slug("rustlang")).await;
        assert!(is_following(&repo, &reader, &slug("rust")).await);
        assert_eq!(followed_tags(&repo, &reader).await, vec![slug("rust")]);
    }

    #[tokio::test]
    async fn following_twice_follows_once() {
        let repo = FakeTagRepo::new();
        let reader = user("reader_01");
        follow_tag(&repo, &reader, &slug("rust")).await;
        follow_tag(&repo, &reader, &slug("rust")).await;
        assert_eq!(followed_tags(&repo, &reader).await.len(), 1);
    }

    #[tokio::test]
    async fn a_tag_knows_who_follows_it() {
        let repo = FakeTagRepo::new();
        let reader = user("reader_01");
        follow_tag(&repo, &reader, &slug("rust")).await;
        assert_eq!(followers_of(&repo, &slug("rust")).await, vec![reader.id()]);
        assert!(followers_of(&repo, &slug("other")).await.is_empty());
    }
}
