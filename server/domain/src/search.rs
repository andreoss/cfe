use crate::{Comment, Topic};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchHit {
    Topic(Topic),
    Comment(Comment),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Body, CommentId, SectionId, TagSet, Title, TopicId, UserId};
    use time::OffsetDateTime;

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
        assert!(matches!(SearchHit::Topic(topic), SearchHit::Topic(_)));
        assert!(matches!(SearchHit::Comment(comment), SearchHit::Comment(_)));
    }
}
