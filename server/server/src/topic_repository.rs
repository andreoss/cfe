use app::TopicRepository;
use domain::{
    Body, Deletion, GroupId, Page, PostScore, Reason, Revision, SectionId, TagSet, Title, Topic,
    TopicId, UserId,
};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgTopicRepository {
    pool: PgPool,
}

impl PgTopicRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub const SELECT_COLUMNS: &str = "id, section_id, author_id, title, body, tags, created_at, \
    postscore, deleted_reason, deleted_by, deleted_at, edited_by, edited_at, group_id, pending, \
    draft, sticky, off_front, resolved, minor";
pub fn aliased_columns(alias: &str) -> String {
    SELECT_COLUMNS
        .split(", ")
        .map(|c| format!("{alias}.{c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(FromRow)]
pub struct TopicRow {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    author_id: uuid::Uuid,
    title: String,
    body: String,
    tags: Vec<String>,
    created_at: OffsetDateTime,
    postscore: i32,
    deleted_reason: Option<String>,
    deleted_by: Option<uuid::Uuid>,
    deleted_at: Option<OffsetDateTime>,
    edited_by: Option<uuid::Uuid>,
    edited_at: Option<OffsetDateTime>,
    group_id: Option<uuid::Uuid>,
    pending: bool,
    draft: bool,
    sticky: bool,
    off_front: bool,
    resolved: bool,
    minor: bool,
}

fn to_revision(row: &TopicRow) -> Option<Revision> {
    match (row.edited_by, row.edited_at) {
        (Some(editor_id), Some(edited_at)) if row.minor => {
            Some(Revision::minor(UserId::new(editor_id), edited_at))
        }
        (Some(editor_id), Some(edited_at)) => {
            Some(Revision::new(UserId::new(editor_id), edited_at))
        }
        _ => None,
    }
}

fn is_minor(topic: &Topic) -> bool {
    topic.revision().map(|r| r.is_minor()).unwrap_or(false)
}

fn to_deletion(row: &TopicRow) -> Option<Deletion> {
    match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            deleted_at,
        )),
        _ => None,
    }
}

pub fn to_topic(row: TopicRow) -> Topic {
    let deleted = to_deletion(&row);
    let edited = to_revision(&row);
    Topic::from_parts(
        TopicId::new(row.id),
        SectionId::new(row.section_id),
        UserId::new(row.author_id),
        Title::parse(&row.title).expect("stored title is valid"),
        Body::parse(&row.body).expect("stored body is valid"),
        TagSet::parse(&row.tags).expect("stored tags are valid"),
        row.created_at,
        deleted,
        edited,
        row.group_id.map(GroupId::new),
        row.pending,
    )
    .with_postscore(PostScore::from_db(row.postscore))
    .with_lifecycle(row.draft, row.sticky, row.off_front, row.resolved)
}

fn tag_strings(topic: &Topic) -> Vec<String> {
    topic
        .tags()
        .as_slice()
        .iter()
        .map(|t| t.as_str().to_owned())
        .collect()
}

#[async_trait::async_trait]
impl TopicRepository for PgTopicRepository {
    async fn save(&self, topic: &Topic) {
        sqlx::query(
            "INSERT INTO topics (id, section_id, author_id, title, body, tags, created_at, \
             group_id, pending, draft, sticky, off_front, resolved, minor) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(topic.id().as_uuid())
        .bind(topic.section_id().as_uuid())
        .bind(topic.author_id().as_uuid())
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(tag_strings(topic))
        .bind(topic.created_at())
        .bind(topic.group_id().map(|g| g.as_uuid()))
        .bind(topic.is_pending())
        .bind(topic.is_draft())
        .bind(topic.is_sticky())
        .bind(topic.is_off_front())
        .bind(topic.is_resolved())
        .bind(is_minor(topic))
        .execute(&self.pool)
        .await
        .expect("insert topic");
    }

    async fn update(&self, topic: &Topic) {
        sqlx::query(
            "UPDATE topics SET title = $2, body = $3, tags = $4, postscore = $5, \
             deleted_reason = $6, deleted_by = $7, deleted_at = $8, edited_by = $9, \
             edited_at = $10, group_id = $11, pending = $12, draft = $13, sticky = $14, \
             off_front = $15, resolved = $16, minor = $17 WHERE id = $1",
        )
        .bind(topic.id().as_uuid())
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(tag_strings(topic))
        .bind(topic.postscore().to_db())
        .bind(topic.deletion().map(|d| d.reason().as_str()))
        .bind(topic.deletion().map(|d| d.moderator_id().as_uuid()))
        .bind(topic.deletion().map(|d| d.deleted_at()))
        .bind(topic.revision().map(|r| r.editor_id().as_uuid()))
        .bind(topic.revision().map(|r| r.edited_at()))
        .bind(topic.group_id().map(|g| g.as_uuid()))
        .bind(topic.is_pending())
        .bind(topic.is_draft())
        .bind(topic.is_sticky())
        .bind(topic.is_off_front())
        .bind(topic.is_resolved())
        .bind(is_minor(topic))
        .execute(&self.pool)
        .await
        .expect("update topic");
    }

    async fn find_by_id(&self, id: TopicId) -> Option<Topic> {
        sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")
        .map(to_topic)
    }

    async fn list_by_section(&self, section_id: SectionId, page: Page) -> Vec<Topic> {
        sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics \
             WHERE section_id = $1 AND deleted_at IS NULL \
             ORDER BY sticky DESC, created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(section_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_section")
        .into_iter()
        .map(to_topic)
        .collect()
    }

    async fn list_by_tag(&self, tag: &domain::Slug, page: Page) -> Vec<Topic> {
        sqlx::query_as::<_, TopicRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics \
             WHERE $1 = ANY(tags) AND deleted_at IS NULL ORDER BY created_at DESC \
             LIMIT $2 OFFSET $3"
        ))
        .bind(tag.as_str())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_tag")
        .into_iter()
        .map(to_topic)
        .collect()
    }
    async fn count_by_section(&self, section_id: SectionId) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM topics WHERE section_id = $1 AND deleted_at IS NULL",
        )
        .bind(section_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count topics by section");
        count as u64
    }

    async fn count_by_tag(&self, tag: &domain::Slug) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM topics WHERE $1 = ANY(tags) AND deleted_at IS NULL",
        )
        .bind(tag.as_str())
        .fetch_one(&self.pool)
        .await
        .expect("count topics by tag");
        count as u64
    }
}
