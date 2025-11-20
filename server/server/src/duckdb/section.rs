use crate::duckdb::conn::Db;
use crate::duckdb::topic::{read_uuid, uuid_value};
use app::SectionRepository;
use domain::{PostScore, Section, SectionId, Slug, Title};
use duckdb::Row;
use duckdb::types::Value;

pub struct DuckSectionRepository {
    db: Db,
}

impl DuckSectionRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const SECTION_COLUMNS: &str = "id, slug, title, topics_score";

struct SectionRow {
    id: uuid::Uuid,
    slug: String,
    title: String,
    topics_score: i32,
}

fn section_row(row: &Row) -> SectionRow {
    SectionRow {
        id: read_uuid(row, 0),
        slug: row.get(1).expect("read slug"),
        title: row.get(2).expect("read title"),
        topics_score: row.get(3).expect("read topics_score"),
    }
}

fn to_section(row: SectionRow) -> Section {
    Section::from_parts(
        SectionId::new(row.id),
        Slug::parse(&row.slug).expect("stored slug is valid"),
        Title::parse(&row.title).expect("stored title is valid"),
        PostScore::from_db(row.topics_score),
    )
}

async fn load_sections(db: &Db, sql: String, params: Vec<Value>) -> Vec<Section> {
    let rows: Vec<SectionRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare sections");
            let mapped = stmt
                .query_map(duckdb::params_from_iter(params.iter()), |row| {
                    Ok(section_row(row))
                })
                .expect("query sections");
            mapped.map(|r| r.expect("read section")).collect()
        })
        .await;
    rows.into_iter().map(to_section).collect()
}

#[async_trait::async_trait]
impl SectionRepository for DuckSectionRepository {
    async fn find_by_slug(&self, slug: &Slug) -> Option<Section> {
        load_sections(
            &self.db,
            format!("SELECT {SECTION_COLUMNS} FROM sections WHERE slug = ?"),
            vec![Value::Text(slug.as_str().to_owned())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn find_by_id(&self, id: SectionId) -> Option<Section> {
        load_sections(
            &self.db,
            format!("SELECT {SECTION_COLUMNS} FROM sections WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list(&self) -> Vec<Section> {
        load_sections(
            &self.db,
            format!("SELECT {SECTION_COLUMNS} FROM sections ORDER BY title"),
            Vec::new(),
        )
        .await
    }
}
