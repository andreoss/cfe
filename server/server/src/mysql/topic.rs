use app::TopicRepository;
use domain::{
    Body, Deletion, Page, PostScore, Reason, Revision, SectionId, Slug, TagSet, Title, Topic,
    TopicId, UserId,
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

pub const TOPIC_COLUMNS: &str = "id, section_id, author_id, title, body, created_at, postscore, \
    deleted_reason, deleted_by, deleted_at, edited_by, edited_at";

#[derive(FromRow)]
pub struct TopicRow {
    pub id: uuid::Uuid,
    pub section_id: uuid::Uuid,
    pub author_id: uuid::Uuid,
    pub title: String,
    pub body: String,
    pub created_at: OffsetDateTime,
    pub postscore: i32,
    pub deleted_reason: Option<String>,
    pub deleted_by: Option<uuid::Uuid>,
    pub deleted_at: Option<OffsetDateTime>,
    pub edited_by: Option<uuid::Uuid>,
    pub edited_at: Option<OffsetDateTime>,
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

pub fn to_topic(row: TopicRow, tags: TagSet) -> Topic {
    let deleted = deletion(&row);
    let edited = revision(row.edited_by, row.edited_at);
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
    )
    .with_postscore(PostScore::from_db(row.postscore))
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
    async fn save(&self, topic: &Topic) {
        sqlx::query(
            "INSERT INTO topics (id, section_id, author_id, title, body, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(topic.id().as_uuid())
        .bind(topic.section_id().as_uuid())
        .bind(topic.author_id().as_uuid())
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(topic.created_at())
        .execute(&self.pool)
        .await
        .expect("insert topic");
        replace_tags(&self.pool, topic).await;
    }

    async fn update(&self, topic: &Topic) {
        sqlx::query(
            "UPDATE topics SET title = ?, body = ?, postscore = ?, deleted_reason = ?, \
             deleted_by = ?, deleted_at = ?, edited_by = ?, edited_at = ? WHERE id = ?",
        )
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(topic.postscore().to_db())
        .bind(topic.deletion().map(|d| d.reason().as_str()))
        .bind(topic.deletion().map(|d| d.moderator_id().as_uuid()))
        .bind(topic.deletion().map(|d| d.deleted_at()))
        .bind(topic.revision().map(|r| r.editor_id().as_uuid()))
        .bind(topic.revision().map(|r| r.edited_at()))
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
             WHERE section_id = ? AND deleted_at IS NULL ORDER BY created_at DESC \
             LIMIT ? OFFSET ?"
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
