use crate::{PostScore, Slug, Title};

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
    topics_score: PostScore,
}

impl Section {
    pub fn new(id: SectionId, slug: Slug, title: Title) -> Self {
        Self {
            id,
            slug,
            title,
            topics_score: PostScore::default(),
        }
    }

    pub fn from_parts(id: SectionId, slug: Slug, title: Title, topics_score: PostScore) -> Self {
        Self {
            id,
            slug,
            title,
            topics_score,
        }
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

    pub fn topics_score(&self) -> PostScore {
        self.topics_score
    }

    pub fn with_topics_score(&self, topics_score: PostScore) -> Self {
        Self {
            topics_score,
            ..self.clone()
        }
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
        assert_eq!(section.topics_score(), PostScore::default());
    }

    #[test]
    fn a_section_carries_a_topic_posting_score() {
        let section = Section::from_parts(
            SectionId::new(uuid::Uuid::nil()),
            Slug::parse("restricted").unwrap(),
            Title::parse("Restricted").unwrap(),
            crate::PostScore::Floor(crate::FLOOR_50),
        );
        assert_eq!(
            section.topics_score(),
            crate::PostScore::Floor(crate::FLOOR_50)
        );
    }

    #[test]
    fn with_topics_score_replaces_only_the_score() {
        let section = Section::new(
            SectionId::new(uuid::Uuid::nil()),
            Slug::parse("general").unwrap(),
            Title::parse("General").unwrap(),
        );
        let restricted = section.with_topics_score(crate::PostScore::Floor(crate::FLOOR_100));
        assert_eq!(restricted.id(), section.id());
        assert_eq!(restricted.slug(), section.slug());
        assert_eq!(
            restricted.topics_score(),
            crate::PostScore::Floor(crate::FLOOR_100)
        );
        assert_eq!(section.topics_score(), PostScore::default());
    }
}
