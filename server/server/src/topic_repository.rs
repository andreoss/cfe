use app::TopicRepository;
use domain::{Body, Deletion, Reason, SectionId, TagSet, Title, Topic, TopicId, UserId};
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

const SELECT_COLUMNS: &str = "id, section_id, author_id, title, body, tags, created_at, \
    deleted_reason, deleted_by, deleted_at";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    author_id: uuid::Uuid,
    title: String,
    body: String,
    tags: Vec<String>,
    created_at: OffsetDateTime,
    deleted_reason: Option<String>,
    deleted_by: Option<uuid::Uuid>,
    deleted_at: Option<OffsetDateTime>,
}

fn to_deletion(row: &Row) -> Option<Deletion> {
    match (&row.deleted_reason, row.deleted_by, row.deleted_at) {
        (Some(reason), Some(moderator_id), Some(deleted_at)) => Some(Deletion::new(
            UserId::new(moderator_id),
            Reason::parse(reason).expect("stored reason is valid"),
            deleted_at,
        )),
        _ => None,
    }
}

fn to_topic(row: Row) -> Topic {
    let deleted = to_deletion(&row);
    Topic::from_parts(
        TopicId::new(row.id),
        SectionId::new(row.section_id),
        UserId::new(row.author_id),
        Title::parse(&row.title).expect("stored title is valid"),
        Body::parse(&row.body).expect("stored body is valid"),
        TagSet::parse(&row.tags).expect("stored tags are valid"),
        row.created_at,
        deleted,
    )
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
            "INSERT INTO topics (id, section_id, author_id, title, body, tags, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(topic.id().as_uuid())
        .bind(topic.section_id().as_uuid())
        .bind(topic.author_id().as_uuid())
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(tag_strings(topic))
        .bind(topic.created_at())
        .execute(&self.pool)
        .await
        .expect("insert topic");
    }

    async fn update(&self, topic: &Topic) {
        sqlx::query(
            "UPDATE topics SET title = $2, body = $3, tags = $4, deleted_reason = $5, \
             deleted_by = $6, deleted_at = $7 WHERE id = $1",
        )
        .bind(topic.id().as_uuid())
        .bind(topic.title().as_str())
        .bind(topic.body().as_str())
        .bind(tag_strings(topic))
        .bind(topic.deletion().map(|d| d.reason().as_str()))
        .bind(topic.deletion().map(|d| d.moderator_id().as_uuid()))
        .bind(topic.deletion().map(|d| d.deleted_at()))
        .execute(&self.pool)
        .await
        .expect("update topic");
    }

    async fn find_by_id(&self, id: TopicId) -> Option<Topic> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")
        .map(to_topic)
    }

    async fn list_by_section(&self, section_id: SectionId) -> Vec<Topic> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics \
             WHERE section_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC"
        ))
        .bind(section_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_section")
        .into_iter()
        .map(to_topic)
        .collect()
    }

    async fn list_by_tag(&self, tag: &domain::Slug) -> Vec<Topic> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM topics \
             WHERE $1 = ANY(tags) AND deleted_at IS NULL ORDER BY created_at DESC"
        ))
        .bind(tag.as_str())
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_tag")
        .into_iter()
        .map(to_topic)
        .collect()
    }
}
