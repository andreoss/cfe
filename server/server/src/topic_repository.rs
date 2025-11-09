use app::TopicRepository;
use domain::{Body, SectionId, Title, Topic, TopicId, UserId};
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

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    author_id: uuid::Uuid,
    title: String,
    body: String,
    created_at: OffsetDateTime,
}

fn to_topic(row: Row) -> Topic {
    Topic::new(
        TopicId::new(row.id),
        SectionId::new(row.section_id),
        UserId::new(row.author_id),
        Title::parse(&row.title).expect("stored title is valid"),
        Body::parse(&row.body).expect("stored body is valid"),
        row.created_at,
    )
}

#[async_trait::async_trait]
impl TopicRepository for PgTopicRepository {
    async fn save(&self, topic: &Topic) {
        sqlx::query(
            "INSERT INTO topics (id, section_id, author_id, title, body, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
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
    }

    async fn find_by_id(&self, id: TopicId) -> Option<Topic> {
        sqlx::query_as::<_, Row>(
            "SELECT id, section_id, author_id, title, body, created_at FROM topics WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_id")
        .map(to_topic)
    }

    async fn list_by_section(&self, section_id: SectionId) -> Vec<Topic> {
        sqlx::query_as::<_, Row>(
            "SELECT id, section_id, author_id, title, body, created_at FROM topics \
             WHERE section_id = $1 ORDER BY created_at DESC",
        )
        .bind(section_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query list_by_section")
        .into_iter()
        .map(to_topic)
        .collect()
    }
}
