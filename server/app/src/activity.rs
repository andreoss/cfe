use crate::ports::{ActivityRepository, EnforcementRepository};
use domain::{ContentItem, UserId};

pub async fn recent_activity(
    activity: &impl ActivityRepository,
    enforcement: &impl EnforcementRepository,
    viewer_id: Option<UserId>,
    limit: u32,
) -> Vec<ContentItem> {
    let items = activity.recent(limit).await;
    let ignored = match viewer_id {
        Some(id) => enforcement.list_ignored(id).await,
        None => Vec::new(),
    };
    if ignored.is_empty() {
        return items;
    }
    items
        .into_iter()
        .filter(|item| {
            let author = match item {
                ContentItem::Topic(topic) => topic.author_id(),
                ContentItem::Comment(comment) => comment.author_id(),
            };
            !ignored.contains(&author)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeActivityRepo, FakeEnforcementRepo};
    use domain::{Body, Comment, CommentId, SectionId, TagSet, Title, Topic, TopicId};
    use time::OffsetDateTime;

    fn author() -> UserId {
        UserId::new(uuid::Uuid::from_u128(1))
    }

    fn other() -> UserId {
        UserId::new(uuid::Uuid::from_u128(2))
    }

    fn topic_by(who: UserId, title: &str) -> ContentItem {
        ContentItem::Topic(Topic::new(
            TopicId::new(uuid::Uuid::new_v4()),
            SectionId::new(uuid::Uuid::nil()),
            who,
            Title::parse(title).unwrap(),
            Body::parse("Body").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        ))
    }

    fn comment_by(who: UserId, body: &str) -> ContentItem {
        ContentItem::Comment(Comment::new(
            CommentId::new(uuid::Uuid::new_v4()),
            TopicId::new(uuid::Uuid::nil()),
            who,
            None,
            Body::parse(body).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        ))
    }

    #[tokio::test]
    async fn returns_what_the_repository_reports() {
        let activity = FakeActivityRepo::with(vec![
            topic_by(author(), "Newest topic"),
            comment_by(other(), "Newest comment"),
        ]);
        let enforcement = FakeEnforcementRepo::new();
        let items = recent_activity(&activity, &enforcement, None, 20).await;
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], ContentItem::Topic(_)));
        assert!(matches!(items[1], ContentItem::Comment(_)));
    }

    #[tokio::test]
    async fn passes_the_limit_through() {
        let activity = FakeActivityRepo::with(Vec::new());
        let enforcement = FakeEnforcementRepo::new();
        recent_activity(&activity, &enforcement, None, 7).await;
        assert_eq!(activity.last_limit(), Some(7));
    }

    #[tokio::test]
    async fn hides_items_from_authors_the_viewer_ignores() {
        let activity = FakeActivityRepo::with(vec![
            topic_by(author(), "From the author"),
            comment_by(other(), "From someone ignored"),
        ]);
        let enforcement = FakeEnforcementRepo::new();
        enforcement.save_ignore(author(), other()).await;
        let items = recent_activity(&activity, &enforcement, Some(author()), 20).await;
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], ContentItem::Topic(_)));
    }

    #[tokio::test]
    async fn an_anonymous_viewer_sees_everything() {
        let activity = FakeActivityRepo::with(vec![
            topic_by(author(), "From the author"),
            comment_by(other(), "From someone ignored"),
        ]);
        let enforcement = FakeEnforcementRepo::new();
        enforcement.save_ignore(author(), other()).await;
        let items = recent_activity(&activity, &enforcement, None, 20).await;
        assert_eq!(items.len(), 2);
    }
}
