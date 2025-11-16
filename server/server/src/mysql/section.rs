use app::SectionRepository;
use domain::{Section, SectionId, Slug, Title};
use sqlx::{FromRow, MySqlPool};

pub struct MySqlSectionRepository {
    pool: MySqlPool,
}

impl MySqlSectionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct SectionRow {
    id: uuid::Uuid,
    slug: String,
    title: String,
}

fn to_section(row: SectionRow) -> Section {
    Section::new(
        SectionId::new(row.id),
        Slug::parse(&row.slug).expect("stored slug is valid"),
        Title::parse(&row.title).expect("stored title is valid"),
    )
}

#[async_trait::async_trait]
impl SectionRepository for MySqlSectionRepository {
    async fn find_by_slug(&self, slug: &Slug) -> Option<Section> {
        sqlx::query_as::<_, SectionRow>("SELECT id, slug, title FROM sections WHERE slug = ?")
            .bind(slug.as_str())
            .fetch_optional(&self.pool)
            .await
            .expect("query find_by_slug")
            .map(to_section)
    }

    async fn find_by_id(&self, id: SectionId) -> Option<Section> {
        sqlx::query_as::<_, SectionRow>("SELECT id, slug, title FROM sections WHERE id = ?")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .expect("query find_by_id")
            .map(to_section)
    }

    async fn list(&self) -> Vec<Section> {
        sqlx::query_as::<_, SectionRow>("SELECT id, slug, title FROM sections ORDER BY title")
            .fetch_all(&self.pool)
            .await
            .expect("query list sections")
            .into_iter()
            .map(to_section)
            .collect()
    }
}
