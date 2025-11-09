#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    User,
    Moderator,
}

impl Role {
    pub fn is_moderator(&self) -> bool {
        matches!(self, Role::Moderator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moderator_is_moderator() {
        assert!(Role::Moderator.is_moderator());
    }

    #[test]
    fn user_is_not_moderator() {
        assert!(!Role::User.is_moderator());
    }
}
