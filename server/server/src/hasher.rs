use app::PasswordHasher;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString},
};
use rand::rngs::OsRng;

pub struct Argon2Hasher;

impl PasswordHasher for Argon2Hasher {
    fn hash(&self, plain: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(plain.as_bytes(), &salt)
            .expect("hash password")
            .to_string()
    }

    fn verify(&self, plain: &str, hash: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_correct_password() {
        let hasher = Argon2Hasher;
        let hash = hasher.hash("correcthorse");
        assert!(hasher.verify("correcthorse", &hash));
    }

    #[test]
    fn rejects_wrong_password() {
        let hasher = Argon2Hasher;
        let hash = hasher.hash("correcthorse");
        assert!(!hasher.verify("wrongpassword", &hash));
    }

    #[test]
    fn rejects_malformed_hash() {
        let hasher = Argon2Hasher;
        assert!(!hasher.verify("anything", "not-a-hash"));
    }

    #[test]
    fn produces_distinct_salts() {
        let hasher = Argon2Hasher;
        let a = hasher.hash("correcthorse");
        let b = hasher.hash("correcthorse");
        assert_ne!(a, b);
    }
}
