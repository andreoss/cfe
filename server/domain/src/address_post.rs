use crate::{Address, ClientString, UserId};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressPost {
    user_id: UserId,
    addr: Address,
    client: Option<ClientString>,
    at: OffsetDateTime,
}

impl AddressPost {
    pub fn new(
        user_id: UserId,
        addr: Address,
        client: Option<ClientString>,
        at: OffsetDateTime,
    ) -> Self {
        Self {
            user_id,
            addr,
            client,
            at,
        }
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn addr(&self) -> &Address {
        &self.addr
    }

    pub fn client(&self) -> Option<&ClientString> {
        self.client.as_ref()
    }

    pub fn at(&self) -> OffsetDateTime {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_who_posted_from_where_and_when() {
        let post = AddressPost::new(
            UserId::new(uuid::Uuid::nil()),
            Address::parse("203.0.113.10").unwrap(),
            Some(ClientString::parse("agent/1.0").unwrap()),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(post.user_id(), UserId::new(uuid::Uuid::nil()));
        assert_eq!(post.addr().as_str(), "203.0.113.10");
        assert_eq!(post.client().map(|c| c.as_str()), Some("agent/1.0"));
        assert_eq!(post.at(), OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn a_client_string_is_optional() {
        let post = AddressPost::new(
            UserId::new(uuid::Uuid::nil()),
            Address::parse("203.0.113.10").unwrap(),
            None,
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(post.client(), None);
    }
}
