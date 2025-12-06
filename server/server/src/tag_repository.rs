use app::TagRepository;
use domain::{Slug, Tag, TagDescription, UserId};
use sqlx::{FromRow, PgPool};

pub struct PgTagRepository {
    pool: PgPool,
}

impl PgTagRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    slug: String,
    description: Option<String>,
    means: Option<String>,
}

fn to_tag(row: Row) -> Option<Tag> {
    let slug = Slug::parse(&row.slug).ok()?;
    let description = match row.description {
        Some(raw) => Some(TagDescription::parse(&raw).ok()?),
        None => None,
    };
    let means = match row.means {
        Some(raw) => Some(Slug::parse(&raw).ok()?),
        None => None,
    };
    Some(Tag::from_parts(slug, description, means))
}

#[async_trait::async_trait]
impl TagRepository for PgTagRepository {
    async fn save(&self, tag: &Tag) {
        sqlx::query(
            "INSERT INTO tags (slug, description, means) VALUES ($1, $2, $3) \
             ON CONFLICT (slug) \
             DO UPDATE SET description = EXCLUDED.description, means = EXCLUDED.means",
        )
        .bind(tag.slug().as_str())
        .bind(tag.description().map(|d| d.as_str()))
        .bind(tag.means().map(|m| m.as_str()))
        .execute(&self.pool)
        .await
        .expect("insert tag");
    }

    async fn find(&self, slug: &Slug) -> Option<Tag> {
        sqlx::query_as::<_, Row>("SELECT slug, description, means FROM tags WHERE slug = $1")
            .bind(slug.as_str())
            .fetch_optional(&self.pool)
            .await
            .expect("query tag")
            .and_then(to_tag)
    }

    async fn list(&self) -> Vec<Tag> {
        sqlx::query_as::<_, Row>("SELECT slug, description, means FROM tags ORDER BY slug")
            .fetch_all(&self.pool)
            .await
            .expect("query tags")
            .into_iter()
            .filter_map(to_tag)
            .collect()
    }

    async fn follow(&self, user_id: UserId, slug: &Slug) {
        sqlx::query(
            "INSERT INTO tag_follows (user_id, slug) VALUES ($1, $2) \
             ON CONFLICT (user_id, slug) DO NOTHING",
        )
        .bind(user_id.as_uuid())
        .bind(slug.as_str())
        .execute(&self.pool)
        .await
        .expect("insert tag follow");
    }

    async fn unfollow(&self, user_id: UserId, slug: &Slug) {
        sqlx::query("DELETE FROM tag_follows WHERE user_id = $1 AND slug = $2")
            .bind(user_id.as_uuid())
            .bind(slug.as_str())
            .execute(&self.pool)
            .await
            .expect("delete tag follow");
    }

    async fn is_following(&self, user_id: UserId, slug: &Slug) -> bool {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tag_follows WHERE user_id = $1 AND slug = $2")
                .bind(user_id.as_uuid())
                .bind(slug.as_str())
                .fetch_one(&self.pool)
                .await
                .expect("count tag follow");
        count > 0
    }

    async fn followed_by(&self, user_id: UserId) -> Vec<Slug> {
        sqlx::query_scalar::<_, String>(
            "SELECT slug FROM tag_follows WHERE user_id = $1 ORDER BY slug",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query followed tags")
        .into_iter()
        .filter_map(|slug| Slug::parse(&slug).ok())
        .collect()
    }

    async fn followers(&self, slug: &Slug) -> Vec<UserId> {
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT user_id FROM tag_follows WHERE slug = $1 ORDER BY user_id",
        )
        .bind(slug.as_str())
        .fetch_all(&self.pool)
        .await
        .expect("query tag followers")
        .into_iter()
        .map(UserId::new)
        .collect()
    }
}
