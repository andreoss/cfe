use axum::http::{HeaderMap, header};

pub const TOKEN_COOKIE: &str = "token";
const TOKEN_LENGTH: usize = 32;

pub struct Guard {
    token: String,
    fresh: bool,
}

impl Guard {
    pub fn new(headers: &HeaderMap) -> Self {
        match cookie_of(headers, TOKEN_COOKIE).filter(|token| is_token(token)) {
            Some(token) => Self {
                token,
                fresh: false,
            },
            None => Self {
                token: new_token(),
                fresh: true,
            },
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn allows(&self, given: Option<&str>) -> bool {
        match given {
            Some(given) => same(given.as_bytes(), self.token.as_bytes()),
            None => false,
        }
    }

    pub fn set_cookie(&self) -> Option<String> {
        self.fresh.then(|| {
            format!(
                "{TOKEN_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax",
                self.token
            )
        })
    }
}

pub fn cookie_of(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())?;
    let prefix = format!("{name}=");
    cookies
        .split(';')
        .map(|pair| pair.trim())
        .find_map(|pair| pair.strip_prefix(&prefix))
        .map(|value| value.to_owned())
}

fn new_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn is_token(raw: &str) -> bool {
    raw.len() == TOKEN_LENGTH && raw.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn same(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (one, other) in left.iter().zip(right.iter()) {
        difference |= one ^ other;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(cookies: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookies.parse().unwrap());
        headers
    }

    #[test]
    fn a_token_is_minted_when_none_comes_in() {
        let guard = Guard::new(&HeaderMap::new());
        assert!(is_token(guard.token()));
        assert!(guard.set_cookie().unwrap().contains(TOKEN_COOKIE));
    }

    #[test]
    fn a_token_that_came_in_is_kept() {
        let token = "0123456789abcdef0123456789abcdef";
        let guard = Guard::new(&headers(&format!("theme=dark; token={token}")));
        assert_eq!(guard.token(), token);
        assert!(guard.set_cookie().is_none());
    }

    #[test]
    fn a_token_that_is_not_the_right_shape_is_replaced() {
        let guard = Guard::new(&headers("token=../etc/passwd"));
        assert!(is_token(guard.token()));
        assert!(guard.set_cookie().is_some());
    }

    #[test]
    fn only_the_token_it_holds_is_allowed() {
        let guard = Guard::new(&HeaderMap::new());
        assert!(guard.allows(Some(guard.token())));
        assert!(!guard.allows(Some("0123456789abcdef0123456789abcdef")));
        assert!(!guard.allows(None));
        assert!(!guard.allows(Some("")));
    }

    #[test]
    fn a_cookie_is_read_by_its_name_only() {
        let headers = headers("theme=dark; token=abc; other=token=xyz");
        assert_eq!(cookie_of(&headers, "token").as_deref(), Some("abc"));
        assert_eq!(cookie_of(&headers, "theme").as_deref(), Some("dark"));
        assert_eq!(cookie_of(&headers, "none"), None);
        assert_eq!(cookie_of(&HeaderMap::new(), "token"), None);
    }
}
