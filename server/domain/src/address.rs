use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address {
    value: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AddressError;

impl Address {
    pub fn parse(input: &str) -> Result<Self, AddressError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AddressError);
        }
        if let Some((host, prefix)) = trimmed.rsplit_once('/') {
            let prefix: u8 = prefix.parse().map_err(|_| AddressError)?;
            let addr: IpAddr = IpAddr::from_str(host).map_err(|_| AddressError)?;
            let max = match addr {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            if prefix > max {
                return Err(AddressError);
            }
        } else {
            IpAddr::from_str(trimmed).map_err(|_| AddressError)?;
        }
        Ok(Self {
            value: trimmed.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_ipv4_address() {
        let address = Address::parse("192.168.0.1").expect("valid address");
        assert_eq!(address.as_str(), "192.168.0.1");
    }

    #[test]
    fn parses_a_plain_ipv6_address() {
        let address = Address::parse("2001:db8::1").expect("valid address");
        assert_eq!(address.as_str(), "2001:db8::1");
    }

    #[test]
    fn parses_a_v4_cidr() {
        let address = Address::parse("10.0.0.0/24").expect("valid cidr");
        assert_eq!(address.as_str(), "10.0.0.0/24");
    }

    #[test]
    fn parses_a_v6_cidr() {
        let address = Address::parse("2001:db8::/32").expect("valid cidr");
        assert_eq!(address.as_str(), "2001:db8::/32");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let address = Address::parse("  203.0.113.5  ").expect("valid address");
        assert_eq!(address.as_str(), "203.0.113.5");
    }

    #[test]
    fn rejects_an_empty_string() {
        assert_eq!(Address::parse(""), Err(AddressError));
        assert_eq!(Address::parse("   "), Err(AddressError));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(Address::parse("not-an-address"), Err(AddressError));
        assert_eq!(Address::parse("999.1.1.1"), Err(AddressError));
    }

    #[test]
    fn rejects_an_out_of_range_cidr_prefix() {
        assert_eq!(Address::parse("192.168.0.0/33"), Err(AddressError));
        assert_eq!(Address::parse("2001:db8::/129"), Err(AddressError));
    }

    #[test]
    fn rejects_a_non_numeric_prefix() {
        assert_eq!(Address::parse("192.168.0.0/wide"), Err(AddressError));
    }
}
