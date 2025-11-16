use app::AvatarRepository;
use domain::{Avatar, ImageFormat, UserId};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

pub struct PgAvatarRepository {
    pool: PgPool,
}

impl PgAvatarRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    bytes: Vec<u8>,
    content_type: String,
}

#[async_trait::async_trait]
impl AvatarRepository for PgAvatarRepository {
    async fn save(&self, user_id: UserId, avatar: &Avatar) {
        sqlx::query(
            "INSERT INTO avatars (user_id, bytes, content_type, updated_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id) DO UPDATE SET bytes = EXCLUDED.bytes, \
             content_type = EXCLUDED.content_type, updated_at = EXCLUDED.updated_at",
        )
        .bind(user_id.as_uuid())
        .bind(avatar.bytes())
        .bind(avatar.format().content_type())
        .bind(OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await
        .expect("insert avatar");
    }

    async fn find_by_user(&self, user_id: UserId) -> Option<Avatar> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT bytes, content_type FROM avatars WHERE user_id = $1",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query avatar")?;
        let format = ImageFormat::parse(&row.content_type)?;
        Some(Avatar::from_parts(row.bytes, format))
    }

    async fn delete(&self, user_id: UserId) {
        sqlx::query("DELETE FROM avatars WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await
            .expect("delete avatar");
    }
}
