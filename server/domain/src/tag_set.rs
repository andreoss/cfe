use crate::{Slug, SlugError};

const MAX_TAGS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSet(Vec<Slug>);

#[derive(Debug, PartialEq, Eq)]
pub enum TagSetError {
    TooMany,
    Invalid(SlugError),
}

impl TagSet {
    pub fn parse(raw: &[String]) -> Result<Self, TagSetError> {
        let mut tags = Vec::new();
        for r in raw {
            let slug = Slug::parse(r).map_err(TagSetError::Invalid)?;
            if !tags.contains(&slug) {
                tags.push(slug);
            }
        }
        if tags.len() > MAX_TAGS {
            return Err(TagSetError::TooMany);
        }
        Ok(Self(tags))
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn as_slice(&self) -> &[Slug] {
        &self.0
    }

    pub fn contains(&self, slug: &Slug) -> bool {
        self.0.contains(slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn parses_distinct_tags() {
        let set = TagSet::parse(&raw(&["rust", "forum"])).unwrap();
        assert_eq!(set.as_slice().len(), 2);
    }

    #[test]
    fn dedups_repeated_tags() {
        let set = TagSet::parse(&raw(&["rust", "rust"])).unwrap();
        assert_eq!(set.as_slice().len(), 1);
    }

    #[test]
    fn empty_has_no_tags() {
        assert_eq!(TagSet::empty().as_slice().len(), 0);
    }

    #[test]
    fn rejects_more_than_max_tags() {
        let set = raw(&["a1", "a2", "a3", "a4", "a5", "a6"]);
        assert_eq!(TagSet::parse(&set), Err(TagSetError::TooMany));
    }

    #[test]
    fn rejects_an_invalid_tag() {
        let set = raw(&["Rust"]);
        assert!(matches!(
            TagSet::parse(&set),
            Err(TagSetError::Invalid(SlugError::InvalidChar('R')))
        ));
    }

    #[test]
    fn contains_checks_membership() {
        let set = TagSet::parse(&raw(&["rust"])).unwrap();
        assert!(set.contains(&Slug::parse("rust").unwrap()));
        assert!(!set.contains(&Slug::parse("forum").unwrap()));
    }
}
