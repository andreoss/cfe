use crate::sqlite::conn::Db;
use crate::sqlite::topic::{read_uuid, uuid_value};
use app::GroupRepository;
use domain::{Group, GroupId, SectionId, Slug, Title};
use rusqlite::Row;
use rusqlite::types::Value;

pub struct SqliteGroupRepository {
    db: Db,
}

impl SqliteGroupRepository {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const GROUP_COLUMNS: &str = "id, section_id, name, slug";

struct GroupRow {
    id: uuid::Uuid,
    section_id: uuid::Uuid,
    name: String,
    slug: String,
}

fn group_row(row: &Row) -> GroupRow {
    GroupRow {
        id: read_uuid(row, 0),
        section_id: read_uuid(row, 1),
        name: row.get(2).expect("read name"),
        slug: row.get(3).expect("read slug"),
    }
}

fn to_group(row: GroupRow) -> Group {
    Group::new(
        GroupId::new(row.id),
        SectionId::new(row.section_id),
        Title::parse(&row.name).expect("stored name is valid"),
        Slug::parse(&row.slug).expect("stored slug is valid"),
    )
}

async fn load_groups(db: &Db, sql: String, params: Vec<Value>) -> Vec<Group> {
    let rows: Vec<GroupRow> = db
        .call(move |conn| {
            let mut stmt = conn.prepare(&sql).expect("prepare groups");
            let mapped = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok(group_row(row))
                })
                .expect("query groups");
            mapped.map(|r| r.expect("read group")).collect()
        })
        .await;
    rows.into_iter().map(to_group).collect()
}

#[async_trait::async_trait]
impl GroupRepository for SqliteGroupRepository {
    async fn save(&self, group: &Group) {
        self.db
            .execute(
                "INSERT INTO groups (id, section_id, name, slug) VALUES (?, ?, ?, ?)",
                vec![
                    uuid_value(group.id().as_uuid()),
                    uuid_value(group.section_id().as_uuid()),
                    Value::Text(group.name().as_str().to_owned()),
                    Value::Text(group.slug().as_str().to_owned()),
                ],
            )
            .await;
    }

    async fn update(&self, group: &Group) {
        self.db
            .execute(
                "UPDATE groups SET name = ? WHERE id = ?",
                vec![
                    Value::Text(group.name().as_str().to_owned()),
                    uuid_value(group.id().as_uuid()),
                ],
            )
            .await;
    }

    async fn find_by_id(&self, id: GroupId) -> Option<Group> {
        load_groups(
            &self.db,
            format!("SELECT {GROUP_COLUMNS} FROM groups WHERE id = ?"),
            vec![uuid_value(id.as_uuid())],
        )
        .await
        .into_iter()
        .next()
    }

    async fn find_by_slug(&self, section_id: SectionId, slug: &Slug) -> Option<Group> {
        load_groups(
            &self.db,
            format!("SELECT {GROUP_COLUMNS} FROM groups WHERE section_id = ? AND slug = ?"),
            vec![
                uuid_value(section_id.as_uuid()),
                Value::Text(slug.as_str().to_owned()),
            ],
        )
        .await
        .into_iter()
        .next()
    }

    async fn list_by_section(&self, section_id: SectionId) -> Vec<Group> {
        load_groups(
            &self.db,
            format!("SELECT {GROUP_COLUMNS} FROM groups WHERE section_id = ? ORDER BY name"),
            vec![uuid_value(section_id.as_uuid())],
        )
        .await
    }
}
