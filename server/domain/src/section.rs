use crate::{Slug, Title};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionId(uuid::Uuid);

impl SectionId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    id: SectionId,
    slug: Slug,
    title: Title,
}

impl Section {
    pub fn new(id: SectionId, slug: Slug, title: Title) -> Self {
        Self { id, slug, title }
    }

    pub fn id(&self) -> SectionId {
        self.id
    }

    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    pub fn title(&self) -> &Title {
        &self.title
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_with_given_fields() {
        let id = SectionId::new(uuid::Uuid::nil());
        let slug = Slug::parse("general-talk").unwrap();
        let title = Title::parse("General Talk").unwrap();
        let section = Section::new(id, slug.clone(), title.clone());
        assert_eq!(section.id(), id);
        assert_eq!(section.slug(), &slug);
        assert_eq!(section.title(), &title);
    }
}
