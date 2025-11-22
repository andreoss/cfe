use crate::{SectionId, Slug, Title};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(uuid::Uuid);

impl GroupId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    id: GroupId,
    section_id: SectionId,
    name: Title,
    slug: Slug,
}

impl Group {
    pub fn new(id: GroupId, section_id: SectionId, name: Title, slug: Slug) -> Self {
        Self {
            id,
            section_id,
            name,
            slug,
        }
    }

    pub fn from_parts(id: GroupId, section_id: SectionId, name: Title, slug: Slug) -> Self {
        Self {
            id,
            section_id,
            name,
            slug,
        }
    }

    pub fn id(&self) -> GroupId {
        self.id
    }

    pub fn section_id(&self) -> SectionId {
        self.section_id
    }

    pub fn name(&self) -> &Title {
        &self.name
    }

    pub fn slug(&self) -> &Slug {
        &self.slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SectionId, Slug, Title};

    #[test]
    fn constructs_with_given_fields() {
        let id = GroupId::new(uuid::Uuid::nil());
        let section_id = SectionId::new(uuid::Uuid::nil());
        let name = Title::parse("Announcements").unwrap();
        let slug = Slug::parse("announcements").unwrap();
        let group = Group::new(id, section_id, name.clone(), slug.clone());
        assert_eq!(group.id(), id);
        assert_eq!(group.section_id(), section_id);
        assert_eq!(group.name(), &name);
        assert_eq!(group.slug(), &slug);
    }

    #[test]
    fn from_parts_carries_every_field() {
        let id = GroupId::new(uuid::Uuid::max());
        let section_id = SectionId::new(uuid::Uuid::max());
        let name = Title::parse("Help Desk").unwrap();
        let slug = Slug::parse("help-desk").unwrap();
        let group = Group::from_parts(id, section_id, name.clone(), slug.clone());
        assert_eq!(group.id(), id);
        assert_eq!(group.section_id(), section_id);
        assert_eq!(group.name(), &name);
        assert_eq!(group.slug(), &slug);
    }
}
