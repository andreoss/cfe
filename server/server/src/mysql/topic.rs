use app::TopicRepository;
use domain::{
    Body, Deletion, GroupId, Page, PostScore, Reason, Revision, SectionId, Slug, TagSet, Title,
    Topic, TopicId, UserId,
};
use sqlx::{FromRow, MySqlPool, Row};
use time::OffsetDateTime;

pub struct MySqlTopicRepository {
    pool: MySqlPool,
}

impl MySqlTopicRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

pub const TOPIC_COLUMNS: &str = "id, section_id, group_id, author_id, title, body, created_at, \
    postscore, pending, deleted_reason, deleted_by, deleted_at, edited_by, edited_at, draft, \
    sticky, off_front, resolved, minor";
#[derive(FromRow)]
pub struct TopicRow {
    pub id: uuid::Uuid,
    pub section_id: uuid::Uuid,
    pub group_id: Option<uuid::Uuid>,
    pub author_id: uuid::Uuid,
    pub title: String,
    pub body: String,
    pub created_at: OffsetDateTime,
    pub postscore: i32,
    pub pending: bool,
    pub deleted_reason: Option<String>,
    pub deleted_by: Option<uuid::Uuid>,
    pub deleted_at: Option<OffsetDateTime>,
    pub edited_by: Option<uuid::Uuid>,
    pub edited_at: Option<OffsetDateTime>,
    pub draft: bool,
    pub sticky: bool,
    pub off_front: bool,
    pub resolved: bool,
    pub minor: bool,
}
pub fn deletion(row: &TopicRow) -> Option<Deletion> {
    match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            deleted_at,
        )),
        _ => None,
    }
}

pub fn revision(by: Option<uuid::Uuid>, at: Option<OffsetDateTime>) -> Option<Revision> {
    match (by, at) {
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

pub fn edit_revision(
    by: Option<uuid::Uuid>,
    at: Option<OffsetDateTime>,
    minor: bool,
) -> Option<Revision> {
    match (by, at) {
        (Some(editor_id), Some(edited_at)) if minor => {
            Some(Revision::minor(UserId::new(editor_id), edited_at))
        }
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

pub fn is_minor(topic: &Topic) -> bool {
    topic.revision().map(|r| r.is_minor()).unwrap_or(false)
}

pub fn to_topic(row: TopicRow, tags: TagSet) -> Topic {
    let deleted = deletion(&row);
    let edited = edit_revision(row.edited_by, row.edited_at, row.minor);
    Topic::from_parts(
        TopicId::new(row.id),
        SectionId::new(row.section_id),
        UserId::new(row.author_id),
        Title::parse(&row.title).expect("stored title is valid"),
        Body::parse(&row.body).expect("stored body is valid"),
        tags,
        row.created_at,
        deleted,
        edited,
        row.group_id.map(GroupId::new),
        row.pending,
    )
    .with_postscore(PostScore::from_db(row.postscore))
    .with_lifecycle(row.draft, row.sticky, row.off_front, row.resolved)
}

pub async fn tags_for(pool: &MySqlPool, topic_id: uuid::Uuid) -> TagSet {
    let rows = sqlx::query("SELECT tag FROM topic_tags WHERE topic_id = ? ORDER BY position")
        .bind(topic_id)
        .fetch_all(pool)
        .await
        .expect("query topic tags");
    let tags: Vec<String> = rows
        .into_iter()
        .map(|r| r.get::<String, _>("tag"))
        .collect();
    TagSet::parse(&tags).expect("stored tags are valid")
}

async fn replace_tags(pool: &MySqlPool, topic: &Topic) {
    sqlx::query("DELETE FROM topic_tags WHERE topic_id = ?")
        .bind(topic.id().as_uuid())
        .execute(pool)
        .await
        .expect("clear topic tags");
    for (position, tag) in topic.tags().as_slice().iter().enumerate() {
        sqlx::query("INSERT INTO topic_tags (topic_id, tag, position) VALUES (?, ?, ?)")
            .bind(topic.id().as_uuid())
            .bind(tag.as_str())
            .bind(position as i32)
            .execute(pool)
            .await
            .expect("insert topic tag");
    }
}

async fn with_tags(pool: &MySqlPool, rows: Vec<TopicRow>) -> Vec<Topic> {
    let mut topics = Vec::with_capacity(rows.len());
    for row in rows {
        let tags = tags_for(pool, row.id).await;
        topics.push(to_topic(row, tags));
    }
    topics
}

#[async_trait::async_trait]
impl TopicRepository for MySqlTopicRepository {
    async fn all_for_archive(&self) -> Vec<Topic> {
        let rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {TOPIC_COLUMNS} FROM topics \
             WHERE deleted_at IS NULL AND draft = 0 AND pending = 0 \
             ORDER BY created_at DESC"
        ))
        .fetch_all(&self.pool)
        .await
        .expect("query all_for_archive");
        with_tags(&self.pool, rows).await
    }

    async fn list_between(
        &self,
        from: OffsetDateTime,
        until: OffsetDateTime,
        page: Page,
    ) -> Vec<Topic> {
        let rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {TOPIC_COLUMNS} FROM topics \
             WHERE created_at >= ? AND created_at < ? \
             AND deleted_at IS NULL AND draft = 0 AND pending = 0 \
             ORDER BY created_at DESC LIMIT ? OFFSET ?"
        ))
        .bind(from)
        .bind(until)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query list_between");
        with_tags(&self.pool, rows).await
    }

    async fn count_between(&self, from: OffsetDateTime, until: OffsetDateTime) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM topics WHERE created_at >= ? AND created_at < ? \
             AND deleted_at IS NULL AND draft = 0 AND pending = 0",
        )
        .bind(from)
        .bind(until)
        .fetch_one(&self.pool)
        .await
        .expect("count topics between");
        count as u64
    }

    async fn save(&self, topic: &Topic) {
        sqlx::query(
            "INSERT INTO topics (id, section_id, group_id, author_id, title, body, created_at, \
             pending, draft, sticky, off_front, resolved, minor) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(topic.id().as_uuid())
        .bind(topic.section_id().as_uuid())
        .bind(topic.group_id().map(|g| g.as_uuid()))
        .bind(topic.author_id().as_uuid())
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(topic.created_at())
        .bind(topic.is_pending())
        .bind(topic.is_draft())
        .bind(topic.is_sticky())
        .bind(topic.is_off_front())
        .bind(topic.is_resolved())
        .bind(is_minor(topic))
        .execute(&self.pool)
        .await
        .expect("insert topic");
        replace_tags(&self.pool, topic).await;
    }

    async fn update(&self, topic: &Topic) {
        sqlx::query(
            "UPDATE topics SET title = ?, body = ?, postscore = ?, pending = ?, \
             deleted_reason = ?, deleted_by = ?, deleted_at = ?, edited_by = ?, edited_at = ?, \
             group_id = ?, draft = ?, sticky = ?, off_front = ?, resolved = ?, minor = ? \
             WHERE id = ?",
        )
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(topic.postscore().to_db())
        .bind(topic.is_pending())
        .bind(topic.deletion().map(|d| d.reason().as_str()))
        .bind(topic.deletion().map(|d| d.moderator_id().as_uuid()))
        .bind(topic.deletion().map(|d| d.deleted_at()))
        .bind(topic.revision().map(|r| r.editor_id().as_uuid()))
        .bind(topic.revision().map(|r| r.edited_at()))
        .bind(topic.group_id().map(|g| g.as_uuid()))
        .bind(topic.is_draft())
        .bind(topic.is_sticky())
        .bind(topic.is_off_front())
        .bind(topic.is_resolved())
        .bind(is_minor(topic))
        .bind(topic.id().as_uuid())
        .execute(&self.pool)
        .await
        .expect("update topic");
        replace_tags(&self.pool, topic).await;
    }

    async fn find_by_id(&self, id: TopicId) -> Option<Topic> {
        let row = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {TOPIC_COLUMNS} FROM topics WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")?;
        let tags = tags_for(&self.pool, row.id).await;
        Some(to_topic(row, tags))
    }

    async fn list_by_section(&self, section_id: SectionId, page: Page) -> Vec<Topic> {
        let rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {TOPIC_COLUMNS} FROM topics \
             WHERE section_id = ? AND deleted_at IS NULL \
             ORDER BY sticky DESC, created_at DESC LIMIT ? OFFSET ?"
        ))
        .bind(section_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_section");
        with_tags(&self.pool, rows).await
    }

    async fn count_by_section(&self, section_id: SectionId) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM topics WHERE section_id = ? AND deleted_at IS NULL",
        )
        .bind(section_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count topics by section");
        count as u64
    }

    async fn list_by_tag(&self, tag: &Slug, page: Page) -> Vec<Topic> {
        let rows = sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {} FROM topics t JOIN topic_tags g ON g.topic_id = t.id \
             WHERE g.tag = ? AND t.deleted_at IS NULL ORDER BY t.created_at DESC \
             LIMIT ? OFFSET ?",
            TOPIC_COLUMNS
                .split(", ")
                .map(|c| format!("t.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .bind(tag.as_str())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_tag");
        with_tags(&self.pool, rows).await
    }

    async fn count_by_tag(&self, tag: &Slug) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM topics t JOIN topic_tags g ON g.topic_id = t.id \
             WHERE g.tag = ? AND t.deleted_at IS NULL",
        )
        .bind(tag.as_str())
        .fetch_one(&self.pool)
        .await
        .expect("count topics by tag");
        count as u64
    }
}
