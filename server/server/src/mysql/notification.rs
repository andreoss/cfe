use app::NotificationRepository;
use domain::{CommentId, Notification, NotificationId, Page, TopicId, UserId};
use sqlx::{FromRow, MySqlPool};
use time::OffsetDateTime;

pub struct MySqlNotificationRepository {
    pool: MySqlPool,
}

impl MySqlNotificationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str =
    "id, recipient_id, actor_id, topic_id, comment_id, created_at, read_at";

#[derive(FromRow)]
struct NotificationRow {
    id: uuid::Uuid,
    recipient_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    topic_id: uuid::Uuid,
    comment_id: uuid::Uuid,
    created_at: OffsetDateTime,
    read_at: Option<OffsetDateTime>,
}

fn to_notification(row: NotificationRow) -> Notification {
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
impl NotificationRepository for MySqlNotificationRepository {
    async fn save(&self, notification: &Notification) {
        sqlx::query(
            "INSERT INTO notifications \
             (id, recipient_id, actor_id, topic_id, comment_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
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
        sqlx::query("UPDATE notifications SET read_at = ? WHERE id = ?")
            .bind(notification.read_at())
            .bind(notification.id().as_uuid())
            .execute(&self.pool)
            .await
            .expect("update notification");
    }

    async fn find_by_id(&self, id: NotificationId) -> Option<Notification> {
        sqlx::query_as::<_, NotificationRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM notifications WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query notification")
        .map(to_notification)
    }

    async fn list_by_recipient(&self, recipient_id: UserId, page: Page) -> Vec<Notification> {
        sqlx::query_as::<_, NotificationRow>(&format!(
            "SELECT {SELECT_COLUMNS} FROM notifications \
             WHERE recipient_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        ))
        .bind(recipient_id.as_uuid())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .expect("query notifications")
        .into_iter()
        .map(to_notification)
        .collect()
    }

    async fn count_by_recipient(&self, recipient_id: UserId) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE recipient_id = ?",
        )
        .bind(recipient_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count notifications");
        count as u64
    }

    async fn count_unread(&self, recipient_id: UserId) -> u64 {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE recipient_id = ? AND read_at IS NULL",
        )
        .bind(recipient_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .expect("count unread notifications");
        count as u64
    }
}
