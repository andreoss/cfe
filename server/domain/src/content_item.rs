use crate::{Comment, Topic};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentItem {
    Topic(Topic),
    Comment(Comment),
}

impl ContentItem {
    pub fn written_at(&self) -> OffsetDateTime {
        match self {
            Self::Topic(topic) => topic.created_at(),
            Self::Comment(comment) => comment.created_at(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Body, CommentId, SectionId, TagSet, Title, TopicId, UserId};

    #[test]
    fn reports_when_either_kind_was_written() {
        let written = OffsetDateTime::UNIX_EPOCH + time::Duration::days(3);
        let topic = ContentItem::Topic(Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            written,
        ));
        let comment = ContentItem::Comment(Comment::new(
            CommentId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Nice topic!").unwrap(),
            written,
        ));
        assert_eq!(topic.written_at(), written);
        assert_eq!(comment.written_at(), written);
    }

    #[test]
    fn holds_either_a_topic_or_a_comment() {
        let topic = Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let comment = Comment::new(
            CommentId::new(uuid::Uuid::nil()),
            TopicId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            None,
            Body::parse("Nice topic!").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(matches!(ContentItem::Topic(topic), ContentItem::Topic(_)));
        assert!(matches!(
            ContentItem::Comment(comment),
            ContentItem::Comment(_)
        ));
    }
}
