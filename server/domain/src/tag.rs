use crate::Slug;

pub const MAX_DESCRIPTION: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDescription(String);

#[derive(Debug, PartialEq, Eq)]
pub enum TagDescriptionError {
    Empty,
    TooLong,
}

impl TagDescription {
    pub fn parse(raw: &str) -> Result<Self, TagDescriptionError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(TagDescriptionError::Empty);
        }
        if trimmed.chars().count() > MAX_DESCRIPTION {
            return Err(TagDescriptionError::TooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    slug: Slug,
    description: Option<TagDescription>,
    means: Option<Slug>,
}

impl Tag {
    pub fn new(slug: Slug) -> Self {
        Self {
            slug,
            description: None,
            means: None,
        }
    }

    pub fn from_parts(
        slug: Slug,
        description: Option<TagDescription>,
        means: Option<Slug>,
    ) -> Self {
        Self {
            slug,
            description,
            means,
        }
    }

    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    pub fn description(&self) -> Option<&TagDescription> {
        self.description.as_ref()
    }

    pub fn means(&self) -> Option<&Slug> {
        self.means.as_ref()
    }

    pub fn is_synonym(&self) -> bool {
        self.means.is_some()
    }

    pub fn described(&self, description: TagDescription) -> Self {
        Self {
            description: Some(description),
            ..self.clone()
        }
    }

    pub fn meaning(&self, other: Slug) -> Option<Self> {
        if other == self.slug {
            return None;
        }
        Some(Self {
            means: Some(other),
            ..self.clone()
        })
    }

    pub fn standing_alone(&self) -> Self {
        Self {
            means: None,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(raw: &str) -> Slug {
        Slug::parse(raw).unwrap()
    }

    #[test]
    fn a_description_is_trimmed_and_kept() {
        let described = TagDescription::parse("  about rust  ").unwrap();
        assert_eq!(described.as_str(), "about rust");
    }

    #[test]
    fn an_empty_description_is_refused() {
        assert_eq!(TagDescription::parse(""), Err(TagDescriptionError::Empty));
        assert_eq!(
            TagDescription::parse("   "),
            Err(TagDescriptionError::Empty)
        );
    }

    #[test]
    fn a_description_longer_than_allowed_is_refused() {
        let long = "a".repeat(MAX_DESCRIPTION + 1);
        assert_eq!(
            TagDescription::parse(&long),
            Err(TagDescriptionError::TooLong)
        );
        assert!(TagDescription::parse(&"a".repeat(MAX_DESCRIPTION)).is_ok());
    }

    #[test]
    fn a_new_tag_says_nothing_and_means_only_itself() {
        let tag = Tag::new(slug("rust"));
        assert_eq!(tag.slug(), &slug("rust"));
        assert_eq!(tag.description(), None);
        assert_eq!(tag.means(), None);
        assert!(!tag.is_synonym());
    }

    #[test]
    fn describing_a_tag_keeps_its_slug() {
        let tag = Tag::new(slug("rust")).described(TagDescription::parse("the language").unwrap());
        assert_eq!(tag.slug(), &slug("rust"));
        assert_eq!(tag.description().map(|d| d.as_str()), Some("the language"));
    }

    #[test]
    fn one_tag_may_mean_another() {
        let tag = Tag::new(slug("rustlang")).meaning(slug("rust")).unwrap();
        assert_eq!(tag.means(), Some(&slug("rust")));
        assert!(tag.is_synonym());
    }

    #[test]
    fn a_tag_may_not_mean_itself() {
        assert_eq!(Tag::new(slug("rust")).meaning(slug("rust")), None);
    }

    #[test]
    fn a_synonym_can_be_made_to_stand_alone_again() {
        let tag = Tag::new(slug("rustlang")).meaning(slug("rust")).unwrap();
        let alone = tag.standing_alone();
        assert!(!alone.is_synonym());
        assert_eq!(alone.slug(), &slug("rustlang"));
    }

    #[test]
    fn describing_a_synonym_keeps_what_it_means() {
        let tag = Tag::new(slug("rustlang"))
            .meaning(slug("rust"))
            .unwrap()
            .described(TagDescription::parse("another name for rust").unwrap());
        assert_eq!(tag.means(), Some(&slug("rust")));
        assert!(tag.description().is_some());
    }
}
