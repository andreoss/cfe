use crate::ports::SearchRepository;
use domain::{Query, ContentItem};

pub async fn search(repo: &(impl SearchRepository + ?Sized), query: &Query) -> Vec<ContentItem> {
    repo.search(query).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeSearchRepo;
    use domain::{Body, Comment, CommentId, SectionId, TagSet, Title, Topic, TopicId, UserId};
    use time::OffsetDateTime;

    fn topic(title: &str, body: &str) -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::new_v4()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse(title).unwrap(),
            Body::parse(body).unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn comment(body: &str) -> Comment {
        Comment::new(
            CommentId::new(uuid::Uuid::new_v4()),
            TopicId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse(body).unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn returns_matching_topics_and_comments_in_rank_order() {
        let repo = FakeSearchRepo::with(vec![
            ContentItem::Topic(topic("Ports and adapters", "A note on layering")),
            ContentItem::Comment(comment("Adapters keep the domain clean")),
        ]);
        let hits = search(&repo, &Query::parse("adapters").unwrap()).await;
        assert_eq!(hits.len(), 2);
        assert!(matches!(hits[0], ContentItem::Topic(_)));
        assert!(matches!(hits[1], ContentItem::Comment(_)));
    }

    #[tokio::test]
    async fn returns_nothing_when_there_is_no_match() {
        let repo = FakeSearchRepo::with(Vec::new());
        let hits = search(&repo, &Query::parse("nothing").unwrap()).await;
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn passes_the_query_through_to_the_repository() {
        let repo = FakeSearchRepo::with(Vec::new());
        search(&repo, &Query::parse("layering").unwrap()).await;
        assert_eq!(repo.last_query(), Some("layering".to_owned()));
    }
}
