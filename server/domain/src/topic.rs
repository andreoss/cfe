use crate::{Body, Deletion, GroupId, PostScore, Revision, SectionId, TagSet, Title, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TopicId(uuid::Uuid);

impl TopicId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    id: TopicId,
    section_id: SectionId,
    author_id: UserId,
    title: Title,
    body: Body,
    tags: TagSet,
    created_at: OffsetDateTime,
    deleted: Option<Deletion>,
    edited: Option<Revision>,
    postscore: PostScore,
    group_id: Option<GroupId>,
    pending: bool,
    draft: bool,
    sticky: bool,
    off_front: bool,
    resolved: bool,
}

impl Topic {
    pub fn new(
        id: TopicId,
        section_id: SectionId,
        author_id: UserId,
        title: Title,
        body: Body,
        tags: TagSet,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            section_id,
            author_id,
            title,
            body,
            tags,
            created_at,
            deleted: None,
            edited: None,
            postscore: PostScore::default(),
            group_id: None,
            pending: false,
            draft: false,
            sticky: false,
            off_front: false,
            resolved: false,
        }
    }

    pub fn from_parts(
        id: TopicId,
        section_id: SectionId,
        author_id: UserId,
        title: Title,
        body: Body,
        tags: TagSet,
        created_at: OffsetDateTime,
        deleted: Option<Deletion>,
        edited: Option<Revision>,
        group_id: Option<GroupId>,
        pending: bool,
    ) -> Self {
        Self {
            id,
            section_id,
            author_id,
            title,
            body,
            tags,
            created_at,
            deleted,
            edited,
            postscore: PostScore::default(),
            group_id,
            pending,
            draft: false,
            sticky: false,
            off_front: false,
            resolved: false,
        }
    }

    pub fn with_lifecycle(
        &self,
        draft: bool,
        sticky: bool,
        off_front: bool,
        resolved: bool,
    ) -> Self {
        Self {
            draft,
            sticky,
            off_front,
            resolved,
            ..self.clone()
        }
    }

    pub fn id(&self) -> TopicId {
        self.id
    }

    pub fn section_id(&self) -> SectionId {
        self.section_id
    }

    pub fn author_id(&self) -> UserId {
        self.author_id
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn tags(&self) -> &TagSet {
        &self.tags
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn deletion(&self) -> Option<&Deletion> {
        self.deleted.as_ref()
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted.is_some()
    }

    pub fn with_deletion(&self, deletion: Deletion) -> Self {
        Self {
            deleted: Some(deletion),
            ..self.clone()
        }
    }

    pub fn revision(&self) -> Option<&Revision> {
        self.edited.as_ref()
    }

    pub fn is_edited(&self) -> bool {
        self.edited.is_some()
    }

    pub fn with_edit(&self, title: Title, body: Body, tags: TagSet, revision: Revision) -> Self {
        Self {
            title,
            body,
            tags,
            edited: Some(revision),
            ..self.clone()
        }
    }

    pub fn postscore(&self) -> PostScore {
        self.postscore
    }

    pub fn with_postscore(&self, postscore: PostScore) -> Self {
        Self {
            postscore,
            ..self.clone()
        }
    }

    pub fn group_id(&self) -> Option<GroupId> {
        self.group_id
    }

    pub fn is_pending(&self) -> bool {
        self.pending
    }

    pub fn is_committed(&self) -> bool {
        !self.pending
    }

    pub fn with_group(&self, group_id: Option<GroupId>) -> Self {
        Self {
            group_id,
            ..self.clone()
        }
    }

    pub fn with_pending(&self, pending: bool) -> Self {
        Self {
            pending,
            ..self.clone()
        }
    }

    pub fn is_draft(&self) -> bool {
        self.draft
    }

    pub fn with_draft(&self, draft: bool) -> Self {
        Self {
            draft,
            ..self.clone()
        }
    }

    pub fn is_sticky(&self) -> bool {
        self.sticky
    }

    pub fn with_sticky(&self, sticky: bool) -> Self {
        Self {
            sticky,
            ..self.clone()
        }
    }

    pub fn is_off_front(&self) -> bool {
        self.off_front
    }

    pub fn with_off_front(&self, off_front: bool) -> Self {
        Self {
            off_front,
            ..self.clone()
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved
    }

    pub fn with_resolved(&self, resolved: bool) -> Self {
        Self {
            resolved,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Reason;

    #[test]
    fn constructs_with_given_fields_and_not_deleted() {
        let id = TopicId::new(uuid::Uuid::nil());
        let section_id = SectionId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let title = Title::parse("Hello").unwrap();
        let body = Body::parse("World").unwrap();
        let tags = TagSet::parse(&["rust".to_string()]).unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        let topic = Topic::new(
            id,
            section_id,
            author_id,
            title.clone(),
            body.clone(),
            tags.clone(),
            now,
        );
        assert_eq!(topic.id(), id);
        assert_eq!(topic.section_id(), section_id);
        assert_eq!(topic.author_id(), author_id);
        assert_eq!(topic.title(), &title);
        assert_eq!(topic.body(), &body);
        assert_eq!(topic.tags(), &tags);
        assert_eq!(topic.created_at(), now);
        assert!(!topic.is_deleted());
        assert_eq!(topic.deletion(), None);
    }

    #[test]
    fn with_edit_replaces_content_and_records_the_revision() {
        let id = TopicId::new(uuid::Uuid::nil());
        let topic = Topic::new(
            id,
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Before").unwrap(),
            Body::parse("Old body").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(!topic.is_edited());
        assert_eq!(topic.revision(), None);
        let title = Title::parse("After").unwrap();
        let body = Body::parse("New body").unwrap();
        let tags = TagSet::parse(&["rust".to_string()]).unwrap();
        let revision = Revision::new(
            UserId::new(uuid::Uuid::max()),
            OffsetDateTime::UNIX_EPOCH,
        );
        let edited = topic.with_edit(title.clone(), body.clone(), tags.clone(), revision.clone());
        assert_eq!(edited.id(), topic.id());
        assert_eq!(edited.created_at(), topic.created_at());
        assert_eq!(edited.title(), &title);
        assert_eq!(edited.body(), &body);
        assert_eq!(edited.tags(), &tags);
        assert!(edited.is_edited());
        assert_eq!(edited.revision(), Some(&revision));
    }

    #[test]
    fn with_postscore_changes_the_comment_restriction() {
        let topic = Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(topic.postscore(), crate::PostScore::default());
        let closed = topic.with_postscore(crate::PostScore::NoComments);
        assert_eq!(closed.id(), topic.id());
        assert_eq!(closed.postscore(), crate::PostScore::NoComments);
        assert_eq!(topic.postscore(), crate::PostScore::default());
    }

    #[test]
    fn with_deletion_marks_deleted_and_keeps_identity() {
        let id = TopicId::new(uuid::Uuid::nil());
        let section_id = SectionId::new(uuid::Uuid::nil());
        let author_id = UserId::new(uuid::Uuid::nil());
        let topic = Topic::new(
            id,
            section_id,
            author_id,
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let deletion = Deletion::new(
            UserId::new(uuid::Uuid::max()),
            Reason::parse("spam").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let deleted = topic.with_deletion(deletion.clone());
        assert_eq!(deleted.id(), topic.id());
        assert!(deleted.is_deleted());
        assert_eq!(deleted.deletion(), Some(&deletion));
    }

    #[test]
    fn a_topic_starts_ungrouped_and_committed() {
        let topic = Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(topic.group_id(), None);
        assert!(topic.is_committed());
        assert!(!topic.is_pending());
    }

    #[test]
    fn with_group_and_with_pending_change_only_those_fields() {
        let topic = Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        );
        let group_id = GroupId::new(uuid::Uuid::max());
        let grouped = topic.with_group(Some(group_id)).with_pending(true);
        assert_eq!(grouped.group_id(), Some(group_id));
        assert!(grouped.is_pending());
        assert!(!grouped.is_committed());
        assert_eq!(grouped.id(), topic.id());
        assert_eq!(topic.group_id(), None);
        assert!(topic.is_committed());
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    fn topic() -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::nil()),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::nil()),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn a_new_topic_carries_no_lifecycle_flags() {
        let topic = topic();
        assert!(!topic.is_draft());
        assert!(!topic.is_sticky());
        assert!(!topic.is_off_front());
        assert!(!topic.is_resolved());
    }

    #[test]
    fn each_flag_can_be_set_on_its_own() {
        let topic = topic();
        assert!(topic.with_draft(true).is_draft());
        assert!(topic.with_sticky(true).is_sticky());
        assert!(topic.with_off_front(true).is_off_front());
        assert!(topic.with_resolved(true).is_resolved());
    }

    #[test]
    fn setting_one_flag_leaves_the_others_alone() {
        let topic = topic().with_sticky(true).with_resolved(true);
        let published = topic.with_draft(false);
        assert!(published.is_sticky());
        assert!(published.is_resolved());
        assert!(!published.is_draft());
        assert_eq!(published.id(), topic.id());
        assert_eq!(published.title(), topic.title());
    }

    #[test]
    fn a_flag_can_be_cleared_again() {
        let topic = topic().with_sticky(true);
        assert!(!topic.with_sticky(false).is_sticky());
    }

    #[test]
    fn lifecycle_restores_every_flag_at_once() {
        let restored = topic().with_lifecycle(true, true, true, true);
        assert!(restored.is_draft());
        assert!(restored.is_sticky());
        assert!(restored.is_off_front());
        assert!(restored.is_resolved());
    }
}
