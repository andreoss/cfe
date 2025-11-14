use app::NotificationRepository;
use domain::{CommentId, Notification, NotificationId, TopicId, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgNotificationRepository {
    pool: PgPool,
}

impl PgNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str =
    "id, recipient_id, actor_id, topic_id, comment_id, created_at, read_at";

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    recipient_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    topic_id: uuid::Uuid,
    comment_id: uuid::Uuid,
    created_at: OffsetDateTime,
    read_at: Option<OffsetDateTime>,
}

fn to_notification(row: Row) -> Notification {
    Notification::from_parts(
        NotificationId::new(row.id),
        UserId::new(row.recipient_id),
        UserId::new(row.actor_id),
        TopicId::new(row.topic_id),
        CommentId::new(row.comment_id),
        row.created_at,
        row.read_at,
    )
}

#[async_trait::async_trait]
impl NotificationRepository for PgNotificationRepository {
    async fn save(&self, notification: &Notification) {
        sqlx::query(
            "INSERT INTO notifications \
             (id, recipient_id, actor_id, topic_id, comment_id, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(notification.id().as_uuid())
        .bind(notification.recipient_id().as_uuid())
        .bind(notification.actor_id().as_uuid())
        .bind(notification.topic_id().as_uuid())
        .bind(notification.comment_id().as_uuid())
        .bind(notification.created_at())
        .execute(&self.pool)
        .await
        .expect("insert notification");
    }

    async fn update(&self, notification: &Notification) {
        sqlx::query("UPDATE notifications SET read_at = $2 WHERE id = $1")
            .bind(notification.id().as_uuid())
            .bind(notification.read_at())
            .execute(&self.pool)
            .await
            .expect("update notification");
    }

    async fn find_by_id(&self, id: NotificationId) -> Option<Notification> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM notifications WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query notification")
        .map(to_notification)
    }

    async fn list_by_recipient(&self, recipient_id: UserId) -> Vec<Notification> {
        sqlx::query_as::<_, Row>(&format!(
            "SELECT {SELECT_COLUMNS} FROM notifications \
             WHERE recipient_id = $1 ORDER BY created_at DESC LIMIT 50"
        ))
        .bind(recipient_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query notifications")
        .into_iter()
        .map(to_notification)
        .collect()
    }

    async fn count_unread(&self, recipient_id: UserId) -> u64 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND read_at IS NULL",
        )
        .bind(recipient_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count unread notifications");
        count as u64
    }
}
