use crate::UserId;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    editor_id: UserId,
    edited_at: OffsetDateTime,
}

impl Revision {
    pub fn new(editor_id: UserId, edited_at: OffsetDateTime) -> Self {
        Self {
            editor_id,
            edited_at,
        }
    }

    pub fn editor_id(&self) -> UserId {
        self.editor_id
    }

    pub fn edited_at(&self) -> OffsetDateTime {
        self.edited_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_exposes_its_fields() {
        let editor_id = UserId::new(uuid::Uuid::nil());
        let now = OffsetDateTime::UNIX_EPOCH;
        let revision = Revision::new(editor_id, now);
        assert_eq!(revision.editor_id(), editor_id);
        assert_eq!(revision.edited_at(), now);
    }
}
