use app::{Challenge, ChallengeRules};
use std::sync::Arc;

pub struct OpenChallenge;

#[async_trait::async_trait]
impl Challenge for OpenChallenge {
    async fn verify(&self, _answer: Option<&str>) -> bool {
        true
    }
}

pub struct SecretChallenge {
    phrase: String,
}

impl SecretChallenge {
    pub fn new(phrase: String) -> Self {
        Self { phrase }
    }
}

#[async_trait::async_trait]
impl Challenge for SecretChallenge {
    async fn verify(&self, answer: Option<&str>) -> bool {
        answer == Some(self.phrase.as_str())
    }
}

fn is_on(raw: Option<&str>) -> bool {
    matches!(raw, Some("1") | Some("true"))
}

fn select(transport: Option<&str>, secret: Option<&str>) -> Arc<dyn Challenge + Send + Sync> {
    match (transport, secret) {
        (Some("secret"), Some(phrase)) if !phrase.is_empty() => {
            Arc::new(SecretChallenge::new(phrase.to_owned()))
        }
        _ => Arc::new(OpenChallenge),
    }
}

fn rules(on_register: Option<&str>, below_floor: Option<&str>) -> ChallengeRules {
    ChallengeRules {
        on_register: is_on(on_register),
        below_floor: is_on(below_floor),
    }
}

pub fn build() -> Arc<dyn Challenge + Send + Sync> {
    let transport = std::env::var("CHALLENGE_TRANSPORT").ok();
    let secret = std::env::var("CHALLENGE_SECRET").ok();
    select(transport.as_deref(), secret.as_deref())
}

pub fn rules_from_env() -> ChallengeRules {
    let on_register = std::env::var("CHALLENGE_ON_REGISTER").ok();
    let below_floor = std::env::var("CHALLENGE_BELOW_FLOOR").ok();
    rules(on_register.as_deref(), below_floor.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_open_challenge_lets_every_answer_through() {
        let challenge = OpenChallenge;
        assert!(challenge.verify(None).await);
        assert!(challenge.verify(Some("")).await);
        assert!(challenge.verify(Some("anything at all")).await);
    }

    #[tokio::test]
    async fn the_secret_challenge_wants_the_phrase_exactly() {
        let challenge = SecretChallenge::new("open sesame".to_owned());
        assert!(challenge.verify(Some("open sesame")).await);
        assert!(!challenge.verify(Some("Open Sesame")).await);
        assert!(!challenge.verify(Some("open sesame ")).await);
        assert!(!challenge.verify(Some("wrong")).await);
        assert!(!challenge.verify(None).await);
    }

    #[tokio::test]
    async fn a_configured_secret_is_the_one_asked_for() {
        let built = select(Some("secret"), Some("open sesame"));
        assert!(built.verify(Some("open sesame")).await);
        assert!(!built.verify(Some("nope")).await);
    }

    #[tokio::test]
    async fn an_unset_transport_stays_open() {
        assert!(select(None, Some("open sesame")).verify(None).await);
        assert!(select(Some("open"), None).verify(None).await);
        assert!(select(Some("something else"), None).verify(None).await);
    }

    #[tokio::test]
    async fn a_secret_transport_without_a_phrase_stays_open() {
        assert!(select(Some("secret"), None).verify(None).await);
        assert!(select(Some("secret"), Some("")).verify(None).await);
    }

    #[test]
    fn the_rules_are_off_until_they_are_asked_for() {
        assert_eq!(rules(None, None), ChallengeRules::default());
        assert_eq!(rules(Some(""), Some("")), ChallengeRules::default());
        assert_eq!(rules(Some("yes"), Some("on")), ChallengeRules::default());
        assert_eq!(rules(Some("0"), Some("false")), ChallengeRules::default());
        assert_eq!(rules(Some("TRUE"), Some("True")), ChallengeRules::default());
    }

    #[test]
    fn each_rule_reads_its_own_variable() {
        assert_eq!(
            rules(Some("1"), None),
            ChallengeRules {
                on_register: true,
                below_floor: false,
            }
        );
        assert_eq!(
            rules(None, Some("true")),
            ChallengeRules {
                on_register: false,
                below_floor: true,
            }
        );
        assert_eq!(
            rules(Some("true"), Some("1")),
            ChallengeRules {
                on_register: true,
                below_floor: true,
            }
        );
    }
}
