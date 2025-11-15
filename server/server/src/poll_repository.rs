use app::PollRepository;
use domain::{Poll, PollId, PollOption, PollOptionId, Question, TopicId, UserId, Vote};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgPollRepository {
    pool: PgPool,
}

impl PgPollRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct PollRow {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    question: String,
    created_at: OffsetDateTime,
}

#[derive(FromRow)]
struct OptionRow {
    id: uuid::Uuid,
    text: String,
}

#[derive(FromRow)]
struct CountRow {
    option_id: uuid::Uuid,
    total: i64,
}

#[async_trait::async_trait]
impl PollRepository for PgPollRepository {
    async fn save(&self, poll: &Poll) {
        sqlx::query("INSERT INTO polls (id, topic_id, question, created_at) VALUES ($1, $2, $3, $4)")
            .bind(poll.id().as_uuid())
            .bind(poll.topic_id().as_uuid())
            .bind(poll.question().as_str())
            .bind(poll.created_at())
            .execute(&self.pool)
            .await
            .expect("insert poll");
        for (position, option) in poll.options().iter().enumerate() {
            sqlx::query(
                "INSERT INTO poll_options (id, poll_id, text, position) VALUES ($1, $2, $3, $4)",
            )
            .bind(option.id().as_uuid())
            .bind(poll.id().as_uuid())
            .bind(option.text().as_str())
            .bind(position as i32)
            .execute(&self.pool)
            .await
            .expect("insert poll option");
        }
    }

    async fn find_by_topic(&self, topic_id: TopicId) -> Option<Poll> {
        let row = sqlx::query_as::<_, PollRow>(
            "SELECT id, topic_id, question, created_at FROM polls WHERE topic_id = $1",
        )
        .bind(topic_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query poll")?;
        let options = sqlx::query_as::<_, OptionRow>(
            "SELECT id, text FROM poll_options WHERE poll_id = $1 ORDER BY position",
        )
        .bind(row.id)
        .fetch_all(&self.pool)
        .await
        .expect("query poll options")
        .into_iter()
        .map(|o| {
            PollOption::new(
                PollOptionId::new(o.id),
                Question::parse(&o.text).expect("stored option is valid"),
            )
        })
        .collect();
        Some(
            Poll::new(
                PollId::new(row.id),
                TopicId::new(row.topic_id),
                Question::parse(&row.question).expect("stored question is valid"),
                options,
                row.created_at,
            )
            .expect("stored poll is valid"),
        )
    }

    async fn save_vote(&self, vote: &Vote) {
        sqlx::query(
            "INSERT INTO poll_votes (poll_id, user_id, option_id, created_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (poll_id, user_id) \
             DO UPDATE SET option_id = EXCLUDED.option_id, created_at = EXCLUDED.created_at",
        )
        .bind(vote.poll_id().as_uuid())
        .bind(vote.user_id().as_uuid())
        .bind(vote.option_id().as_uuid())
        .bind(vote.created_at())
        .execute(&self.pool)
        .await
        .expect("insert vote");
    }

    async fn counts(&self, poll_id: PollId) -> Vec<(PollOptionId, u64)> {
        sqlx::query_as::<_, CountRow>(
            "SELECT o.id AS option_id, COUNT(v.user_id) AS total \
             FROM poll_options o LEFT JOIN poll_votes v ON v.option_id = o.id \
             WHERE o.poll_id = $1 GROUP BY o.id, o.position ORDER BY o.position",
        )
        .bind(poll_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("count votes")
        .into_iter()
        .map(|row| (PollOptionId::new(row.option_id), row.total as u64))
        .collect()
    }

    async fn find_vote(&self, poll_id: PollId, user_id: UserId) -> Option<PollOptionId> {
        let option_id: Option<uuid::Uuid> = sqlx::query_scalar(
            "SELECT option_id FROM poll_votes WHERE poll_id = $1 AND user_id = $2",
        )
        .bind(poll_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query own vote");
        option_id.map(PollOptionId::new)
    }
}
