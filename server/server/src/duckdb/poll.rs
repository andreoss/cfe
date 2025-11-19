use crate::duckdb::conn::Db;
use crate::duckdb::topic::{read_time, read_uuid, time_to_value, uuid_value};
use app::PollRepository;
use domain::{Poll, PollId, PollOption, PollOptionId, Question, TopicId, UserId, Vote};
use duckdb::types::Value;
use time::OffsetDateTime;

pub struct DuckPollRepository {
    db: Db,
}

impl DuckPollRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

struct PollRow {
    id: uuid::Uuid,
    topic_id: uuid::Uuid,
    question: String,
    created_at: OffsetDateTime,
}

#[async_trait::async_trait]
impl PollRepository for DuckPollRepository {
    async fn save(&self, poll: &Poll) {
        let params = vec![
            uuid_value(poll.id().as_uuid()),
            uuid_value(poll.topic_id().as_uuid()),
            Value::Text(poll.question().as_str().to_owned()),
            time_to_value(poll.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO polls (id, topic_id, question, created_at) VALUES (?, ?, ?, ?)",
                params,
            )
            .await;
        for (position, option) in poll.options().iter().enumerate() {
            let params = vec![
                uuid_value(option.id().as_uuid()),
                uuid_value(poll.id().as_uuid()),
                Value::Text(option.text().as_str().to_owned()),
                Value::Int(position as i32),
            ];
            self.db
                .execute(
                    "INSERT INTO poll_options (id, poll_id, text, position) VALUES (?, ?, ?, ?)",
                    params,
                )
                .await;
        }
    }

    async fn find_by_topic(&self, topic_id: TopicId) -> Option<Poll> {
        let id = topic_id.as_uuid();
        let rows: Vec<PollRow> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, topic_id, question, created_at FROM polls WHERE topic_id = ?",
                    )
                    .expect("prepare poll");
                let mapped = stmt
                    .query_map([uuid_value(id)], |row| {
                        Ok(PollRow {
                            id: read_uuid(row, 0),
                            topic_id: read_uuid(row, 1),
                            question: row.get(2).expect("read question"),
                            created_at: read_time(row, 3),
                        })
                    })
                    .expect("query poll");
                mapped.map(|r| r.expect("read poll")).collect()
            })
            .await;
        let row = rows.into_iter().next()?;
        let poll_id = row.id;
        let options: Vec<PollOption> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, text FROM poll_options WHERE poll_id = ? ORDER BY position",
                    )
                    .expect("prepare poll options");
                let mapped = stmt
                    .query_map([uuid_value(poll_id)], |row| {
                        Ok((
                            read_uuid(row, 0),
                            row.get::<_, String>(1).expect("read text"),
                        ))
                    })
                    .expect("query poll options");
                mapped
                    .map(|r| r.expect("read poll option"))
                    .map(|(id, text)| {
                        PollOption::new(
                            PollOptionId::new(id),
                            Question::parse(&text).expect("stored option is valid"),
                        )
                    })
                    .collect()
            })
            .await;
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
        let params = vec![
            uuid_value(vote.poll_id().as_uuid()),
            uuid_value(vote.user_id().as_uuid()),
            uuid_value(vote.option_id().as_uuid()),
            time_to_value(vote.created_at()),
        ];
        self.db
            .execute(
                "INSERT INTO poll_votes (poll_id, user_id, option_id, created_at) \
                 VALUES (?, ?, ?, ?) \
                 ON CONFLICT (poll_id, user_id) \
                 DO UPDATE SET option_id = EXCLUDED.option_id, created_at = EXCLUDED.created_at",
                params,
            )
            .await;
    }

    async fn counts(&self, poll_id: PollId) -> Vec<(PollOptionId, u64)> {
        let id = poll_id.as_uuid();
        let rows: Vec<(uuid::Uuid, i64)> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT o.id AS option_id, COUNT(v.user_id) AS total \
                         FROM poll_options o LEFT JOIN poll_votes v ON v.option_id = o.id \
                         WHERE o.poll_id = ? GROUP BY o.id, o.position ORDER BY o.position",
                    )
                    .expect("prepare vote counts");
                let mapped = stmt
                    .query_map([uuid_value(id)], |row| {
                        Ok((read_uuid(row, 0), row.get::<_, i64>(1).expect("read total")))
                    })
                    .expect("count votes");
                mapped.map(|r| r.expect("read count")).collect()
            })
            .await;
        rows.into_iter()
            .map(|(option_id, total)| (PollOptionId::new(option_id), total as u64))
            .collect()
    }

    async fn find_vote(&self, poll_id: PollId, user_id: UserId) -> Option<PollOptionId> {
        let params = vec![uuid_value(poll_id.as_uuid()), uuid_value(user_id.as_uuid())];
        let rows: Vec<uuid::Uuid> = self
            .db
            .call(move |conn| {
                let mut stmt = conn
                    .prepare("SELECT option_id FROM poll_votes WHERE poll_id = ? AND user_id = ?")
                    .expect("prepare own vote");
                let mapped = stmt
                    .query_map(duckdb::params_from_iter(params.iter()), |row| {
                        Ok(read_uuid(row, 0))
                    })
                    .expect("query own vote");
                mapped.map(|r| r.expect("read vote")).collect()
            })
            .await;
        rows.into_iter().next().map(PollOptionId::new)
    }
}
