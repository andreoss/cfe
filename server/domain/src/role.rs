#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    User,
    Corrector,
    Moderator,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RoleError;

impl Role {
    pub fn parse(raw: &str) -> Result<Self, RoleError> {
        match raw {
            "user" => Ok(Self::User),
            "corrector" => Ok(Self::Corrector),
            "moderator" => Ok(Self::Moderator),
            _ => Err(RoleError),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Corrector => "corrector",
            Self::Moderator => "moderator",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::User, Self::Corrector, Self::Moderator]
    }

    pub fn is_moderator(&self) -> bool {
        matches!(self, Role::Moderator)
    }

    pub fn is_corrector(&self) -> bool {
        matches!(self, Role::Corrector)
    }

    pub fn may_correct(&self) -> bool {
        matches!(self, Role::Corrector | Role::Moderator)
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

    #[test]
    fn a_corrector_is_not_a_moderator() {
        assert!(!Role::Corrector.is_moderator());
        assert!(Role::Corrector.is_corrector());
    }

    #[test]
    fn a_corrector_and_a_moderator_may_correct_but_a_reader_may_not() {
        assert!(Role::Corrector.may_correct());
        assert!(Role::Moderator.may_correct());
        assert!(!Role::User.may_correct());
    }

    #[test]
    fn parses_every_role_round_trip() {
        for role in Role::all() {
            assert_eq!(Role::parse(role.as_str()), Ok(role));
        }
    }

    #[test]
    fn rejects_an_unknown_role() {
        assert_eq!(Role::parse("admin"), Err(RoleError));
        assert_eq!(Role::parse(""), Err(RoleError));
        assert_eq!(Role::parse("Moderator"), Err(RoleError));
    }
}
