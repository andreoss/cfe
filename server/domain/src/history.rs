use crate::{Body, Title, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionId(uuid::Uuid);

impl VersionId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionOf {
    Topic,
    Comment,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VersionOfError;

impl VersionOf {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Topic => "topic",
            Self::Comment => "comment",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, VersionOfError> {
        match raw {
            "topic" => Ok(Self::Topic),
            "comment" => Ok(Self::Comment),
            _ => Err(VersionOfError),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    id: VersionId,
    of: VersionOf,
    subject_id: uuid::Uuid,
    title: Option<Title>,
    body: Body,
    editor_id: UserId,
    written_at: OffsetDateTime,
}

impl Version {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: VersionId,
        of: VersionOf,
        subject_id: uuid::Uuid,
        title: Option<Title>,
        body: Body,
        editor_id: UserId,
        written_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            of,
            subject_id,
            title,
            body,
            editor_id,
            written_at,
        }
    }

    pub fn id(&self) -> VersionId {
        self.id
    }

    pub fn of(&self) -> VersionOf {
        self.of
    }

    pub fn subject_id(&self) -> uuid::Uuid {
        self.subject_id
    }

    pub fn title(&self) -> Option<&Title> {
        self.title.as_ref()
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn editor_id(&self) -> UserId {
        self.editor_id
    }

    pub fn written_at(&self) -> OffsetDateTime {
        self.written_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Kept(String),
    Removed(String),
    Added(String),
}

pub fn difference(before: &str, after: &str) -> Vec<Change> {
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    let common = longest_common(&old, &new);
    let mut changes = Vec::new();
    let mut i = 0;
    let mut j = 0;
    for step in common {
        while i < step.0 {
            changes.push(Change::Removed(old[i].to_owned()));
            i += 1;
        }
        while j < step.1 {
            changes.push(Change::Added(new[j].to_owned()));
            j += 1;
        }
        changes.push(Change::Kept(old[i].to_owned()));
        i += 1;
        j += 1;
    }
    while i < old.len() {
        changes.push(Change::Removed(old[i].to_owned()));
        i += 1;
    }
    while j < new.len() {
        changes.push(Change::Added(new[j].to_owned()));
        j += 1;
    }
    changes
}

fn longest_common(old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
    let rows = old.len() + 1;
    let columns = new.len() + 1;
    let mut lengths = vec![0usize; rows * columns];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lengths[i * columns + j] = if old[i] == new[j] {
                lengths[(i + 1) * columns + j + 1] + 1
            } else {
                lengths[(i + 1) * columns + j].max(lengths[i * columns + j + 1])
            };
        }
    }
    let mut pairs = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if lengths[(i + 1) * columns + j] >= lengths[i * columns + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(changes: &[Change]) -> Vec<String> {
        changes
            .iter()
            .map(|c| match c {
                Change::Kept(l) => format!("  {l}"),
                Change::Removed(l) => format!("- {l}"),
                Change::Added(l) => format!("+ {l}"),
            })
            .collect()
    }

    #[test]
    fn a_kind_round_trips_through_its_name() {
        for kind in [VersionOf::Topic, VersionOf::Comment] {
            assert_eq!(VersionOf::parse(kind.as_str()), Ok(kind));
        }
        assert_eq!(VersionOf::parse("something"), Err(VersionOfError));
    }

    #[test]
    fn nothing_changed_is_all_kept() {
        let changes = difference("one\ntwo", "one\ntwo");
        assert_eq!(lines(&changes), vec!["  one", "  two"]);
    }

    #[test]
    fn a_line_added_at_the_end() {
        let changes = difference("one", "one\ntwo");
        assert_eq!(lines(&changes), vec!["  one", "+ two"]);
    }

    #[test]
    fn a_line_removed_from_the_end() {
        let changes = difference("one\ntwo", "one");
        assert_eq!(lines(&changes), vec!["  one", "- two"]);
    }

    #[test]
    fn a_line_changed_reads_as_one_out_and_one_in() {
        let changes = difference("one\ntwo\nthree", "one\ntwo and a half\nthree");
        assert_eq!(
            lines(&changes),
            vec!["  one", "- two", "+ two and a half", "  three"]
        );
    }

    #[test]
    fn a_line_added_in_the_middle() {
        let changes = difference("one\nthree", "one\ntwo\nthree");
        assert_eq!(lines(&changes), vec!["  one", "+ two", "  three"]);
    }

    #[test]
    fn everything_replaced() {
        let changes = difference("one\ntwo", "three\nfour");
        assert_eq!(lines(&changes), vec!["- one", "- two", "+ three", "+ four"]);
    }

    #[test]
    fn from_nothing_to_something() {
        assert_eq!(lines(&difference("", "one")), vec!["+ one"]);
    }

    #[test]
    fn from_something_to_nothing() {
        assert_eq!(lines(&difference("one", "")), vec!["- one"]);
    }

    #[test]
    fn nothing_either_side_is_no_change_at_all() {
        assert!(difference("", "").is_empty());
    }

    #[test]
    fn a_repeated_line_is_not_confused_with_its_twin() {
        let changes = difference("a\nb\na", "a\nb\na\nb\na");
        assert_eq!(
            changes
                .iter()
                .filter(|c| matches!(c, Change::Kept(_)))
                .count(),
            3
        );
        assert_eq!(
            changes
                .iter()
                .filter(|c| matches!(c, Change::Added(_)))
                .count(),
            2
        );
        assert!(!changes.iter().any(|c| matches!(c, Change::Removed(_))));
    }

    #[test]
    fn a_version_carries_what_was_written_and_by_whom() {
        let version = Version::new(
            VersionId::new(uuid::Uuid::nil()),
            VersionOf::Topic,
            uuid::Uuid::from_u128(7),
            Some(Title::parse("A subject").unwrap()),
            Body::parse("A body.").unwrap(),
            UserId::new(uuid::Uuid::from_u128(1)),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(version.of(), VersionOf::Topic);
        assert_eq!(version.subject_id(), uuid::Uuid::from_u128(7));
        assert_eq!(version.title().map(|t| t.as_str()), Some("A subject"));
        assert_eq!(version.body().as_str(), "A body.");
        assert_eq!(version.written_at(), OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn a_comment_version_has_no_title() {
        let version = Version::new(
            VersionId::new(uuid::Uuid::nil()),
            VersionOf::Comment,
            uuid::Uuid::from_u128(7),
            None,
            Body::parse("A body.").unwrap(),
            UserId::new(uuid::Uuid::from_u128(1)),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(version.title(), None);
        assert_eq!(version.of(), VersionOf::Comment);
    }
}
