use crate::Query;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Everything,
    Topics,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Relevance,
    Newest,
    Oldest,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CriteriaError {
    UnknownScope,
    UnknownOrder,
}

impl Scope {
    pub fn parse(raw: &str) -> Result<Self, CriteriaError> {
        match raw {
            "everything" => Ok(Self::Everything),
            "topics" => Ok(Self::Topics),
            "comments" => Ok(Self::Comments),
            _ => Err(CriteriaError::UnknownScope),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Everything => "everything",
            Self::Topics => "topics",
            Self::Comments => "comments",
        }
    }

    pub fn covers_topics(self) -> bool {
        matches!(self, Self::Everything | Self::Topics)
    }

    pub fn covers_comments(self) -> bool {
        matches!(self, Self::Everything | Self::Comments)
    }
}

impl Order {
    pub fn parse(raw: &str) -> Result<Self, CriteriaError> {
        match raw {
            "relevance" => Ok(Self::Relevance),
            "newest" => Ok(Self::Newest),
            "oldest" => Ok(Self::Oldest),
            _ => Err(CriteriaError::UnknownOrder),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Newest => "newest",
            Self::Oldest => "oldest",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criteria {
    query: Query,
    scope: Scope,
    order: Order,
}

impl Criteria {
    pub fn new(query: Query, scope: Scope, order: Order) -> Self {
        Self {
            query,
            scope,
            order,
        }
    }

    pub fn parse(
        query: Query,
        scope: Option<&str>,
        order: Option<&str>,
    ) -> Result<Self, CriteriaError> {
        let scope = match scope {
            Some(raw) => Scope::parse(raw)?,
            None => Scope::Everything,
        };
        let order = match order {
            Some(raw) => Order::parse(raw)?,
            None => Order::Relevance,
        };
        Ok(Self::new(query, scope, order))
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    pub fn scope(&self) -> Scope {
        self.scope
    }

    pub fn order(&self) -> Order {
        self.order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> Query {
        Query::parse("anything").unwrap()
    }

    #[test]
    fn reads_every_scope_it_offers() {
        for scope in [Scope::Everything, Scope::Topics, Scope::Comments] {
            assert_eq!(Scope::parse(scope.label()), Ok(scope));
        }
    }

    #[test]
    fn reads_every_order_it_offers() {
        for order in [Order::Relevance, Order::Newest, Order::Oldest] {
            assert_eq!(Order::parse(order.label()), Ok(order));
        }
    }

    #[test]
    fn refuses_a_scope_it_does_not_know() {
        assert_eq!(Scope::parse("drafts"), Err(CriteriaError::UnknownScope));
        assert_eq!(Scope::parse(""), Err(CriteriaError::UnknownScope));
        assert_eq!(Scope::parse("Topics"), Err(CriteriaError::UnknownScope));
    }

    #[test]
    fn refuses_an_order_it_does_not_know() {
        assert_eq!(Order::parse("best"), Err(CriteriaError::UnknownOrder));
        assert_eq!(Order::parse(""), Err(CriteriaError::UnknownOrder));
    }

    #[test]
    fn everything_covers_both_kinds() {
        assert!(Scope::Everything.covers_topics());
        assert!(Scope::Everything.covers_comments());
    }

    #[test]
    fn a_narrowed_scope_covers_one_kind_only() {
        assert!(Scope::Topics.covers_topics());
        assert!(!Scope::Topics.covers_comments());
        assert!(Scope::Comments.covers_comments());
        assert!(!Scope::Comments.covers_topics());
    }

    #[test]
    fn asking_for_nothing_in_particular_searches_everything_by_relevance() {
        let criteria = Criteria::parse(query(), None, None).unwrap();
        assert_eq!(criteria.scope(), Scope::Everything);
        assert_eq!(criteria.order(), Order::Relevance);
    }

    #[test]
    fn a_narrowing_it_does_not_know_is_refused_rather_than_ignored() {
        assert_eq!(
            Criteria::parse(query(), Some("everywhere"), None),
            Err(CriteriaError::UnknownScope)
        );
        assert_eq!(
            Criteria::parse(query(), None, Some("loudest")),
            Err(CriteriaError::UnknownOrder)
        );
    }

    #[test]
    fn keeps_what_it_was_asked_for() {
        let criteria = Criteria::parse(query(), Some("comments"), Some("oldest")).unwrap();
        assert_eq!(criteria.scope(), Scope::Comments);
        assert_eq!(criteria.order(), Order::Oldest);
        assert_eq!(criteria.query().as_str(), "anything");
    }
}
