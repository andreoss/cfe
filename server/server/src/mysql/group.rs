use app::GroupRepository;
use domain::{Group, GroupId, SectionId, Slug, Title};
use sqlx::{FromRow, MySqlPool};

pub struct MySqlGroupRepository {
    pool: MySqlPool,
}

impl MySqlGroupRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct GroupRow {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    name: String,
    slug: String,
}

fn to_group(row: GroupRow) -> Group {
    Group::new(
        GroupId::new(row.id),
        SectionId::new(row.section_id),
        Title::parse(&row.name).expect("stored name is valid"),
        Slug::parse(&row.slug).expect("stored slug is valid"),
    )
}

#[async_trait::async_trait]
impl GroupRepository for MySqlGroupRepository {
    async fn save(&self, group: &Group) {
        sqlx::query("INSERT INTO `groups` (id, section_id, name, slug) VALUES (?, ?, ?, ?)")
            .bind(group.id().as_uuid())
            .bind(group.section_id().as_uuid())
            .bind(group.name().as_str())
            .bind(group.slug().as_str())
            .execute(&self.pool)
            .await
            .expect("insert group");
    }

    async fn find_by_id(&self, id: GroupId) -> Option<Group> {
        sqlx::query_as::<_, GroupRow>(
            "SELECT id, section_id, name, slug FROM `groups` WHERE id = ?",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .expect("query find group by id")
        .map(to_group)
    }

    async fn find_by_slug(&self, section_id: SectionId, slug: &Slug) -> Option<Group> {
        sqlx::query_as::<_, GroupRow>(
            "SELECT id, section_id, name, slug FROM `groups` WHERE section_id = ? AND slug = ?",
        )
        .bind(section_id.as_uuid())
        .bind(slug.as_str())
        .fetch_optional(&self.pool)
        .await
        .expect("query find group by slug")
        .map(to_group)
    }

    async fn list_by_section(&self, section_id: SectionId) -> Vec<Group> {
        sqlx::query_as::<_, GroupRow>(
            "SELECT id, section_id, name, slug FROM `groups` WHERE section_id = ? ORDER BY name",
        )
        .bind(section_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("query list groups by section")
        .into_iter()
        .map(to_group)
        .collect()
    }
}
