use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Section {
    pub slug: String,
    pub title: String,
    pub topics_score: String,
    pub may_post: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClientError {
    BadBaseUrl(String),
    Transport(String),
    Status(u16),
    Detail(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::BadBaseUrl(raw) => {
                write!(f, "the server address is not an http address: {raw}")
            }
            ClientError::Transport(detail) => write!(f, "the server could not be reached: {detail}"),
            ClientError::Status(status) => write!(f, "the server answered {status}"),
            ClientError::Detail(detail) => write!(f, "the server sent an unreadable answer: {detail}"),
        }
    }
}

impl std::error::Error for ClientError {}

#[derive(Clone)]
pub struct ApiClient {
    base: String,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base: &str) -> Result<Self, ClientError> {
        let trimmed = base.trim_end_matches('/').to_owned();
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            return Err(ClientError::BadBaseUrl(base.to_owned()));
        }
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(Self { base: trimmed, http })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub async fn sections(&self) -> Result<Vec<Section>, ClientError> {
        self.get("/api/sections").await
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ClientError> {
        let response = self
            .http
            .get(format!("{}{}", self.base, path))
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(ClientError::Status(status));
        }
        response
            .json()
            .await
            .map_err(|e| ClientError::Detail(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_http_address_is_accepted_and_kept_without_a_trailing_slash() {
        let client = ApiClient::new("http://127.0.0.1:8080/").unwrap();
        assert_eq!(client.base(), "http://127.0.0.1:8080");
    }

    #[test]
    fn an_address_that_is_not_http_is_refused() {
        match ApiClient::new("postgres://localhost/tcbs") {
            Err(ClientError::BadBaseUrl(raw)) => {
                assert_eq!(raw, "postgres://localhost/tcbs".to_owned())
            }
            _ => panic!("an address that is not http must be refused"),
        }
    }

    #[test]
    fn an_empty_address_is_refused() {
        assert!(ApiClient::new("").is_err());
    }

    #[test]
    fn errors_are_described_in_words() {
        assert_eq!(
            ClientError::Status(503).to_string(),
            "the server answered 503"
        );
        assert_eq!(
            ClientError::BadBaseUrl("ftp://x".to_owned()).to_string(),
            "the server address is not an http address: ftp://x"
        );
        assert!(ClientError::Transport("timeout".to_owned())
            .to_string()
            .contains("timeout"));
        assert!(ClientError::Detail("bad json".to_owned())
            .to_string()
            .contains("bad json"));
    }
}
