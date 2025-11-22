use app::SectionRepository;
use domain::{PostScore, Section, SectionId, Slug, Title};
use sqlx::{FromRow, PgPool};

pub struct PgSectionRepository {
    pool: PgPool,
}

impl PgSectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    slug: String,
    title: String,
    topics_score: i32,
}

fn to_section(row: Row) -> Section {
    Section::from_parts(
        SectionId::new(row.id),
        Slug::parse(&row.slug).expect("stored slug is valid"),
        Title::parse(&row.title).expect("stored title is valid"),
        PostScore::from_db(row.topics_score),
    )
}

#[async_trait::async_trait]
impl SectionRepository for PgSectionRepository {
    async fn find_by_slug(&self, slug: &Slug) -> Option<Section> {
        sqlx::query_as::<_, Row>(
            "SELECT id, slug, title, topics_score FROM sections WHERE slug = $1",
        )
        .bind(slug.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find_by_slug")
        .map(to_section)
    }

    async fn find_by_id(&self, id: SectionId) -> Option<Section> {
        sqlx::query_as::<_, Row>("SELECT id, slug, title, topics_score FROM sections WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .expect("query find_by_id")
            .map(to_section)
    }

    async fn list(&self) -> Vec<Section> {
        sqlx::query_as::<_, Row>(
            "SELECT id, slug, title, topics_score FROM sections ORDER BY title",
        )
        .fetch_all(&self.pool)
        .await
        .expect("query list sections")
        .into_iter()
        .map(to_section)
        .collect()
    }
}
