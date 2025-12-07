use crate::ports::{
    MailTokenRepository, Mailer, Message, PasswordHasher, SessionRepository, TokenDigest,
    UserRepository,
};
use domain::{Email, MailToken, MailTokenId, Password, TokenPurpose, User, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum RedeemError {
    Unusable,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeEmailError {
    AddressTaken,
}

async fn issue(
    tokens: &(impl MailTokenRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    id: MailTokenId,
    user_id: UserId,
    purpose: TokenPurpose,
    secret: String,
    payload: Option<String>,
    now: OffsetDateTime,
) -> String {
    let token = MailToken::issue(id, user_id, purpose, digester.digest(&secret), payload, now);
    tokens.save(&token).await;
    secret
}

pub async fn request_password_reset(
    users: &(impl UserRepository + ?Sized),
    tokens: &(impl MailTokenRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    mailer: &(impl Mailer + ?Sized),
    id: MailTokenId,
    address: &Email,
    secret: String,
    now: OffsetDateTime,
) {
    let Some(user) = users.find_by_email(address).await.filter(|u| u.is_active()) else {
        return;
    };
    let secret = issue(
        tokens,
        digester,
        id,
        user.id(),
        TokenPurpose::PasswordReset,
        secret,
        None,
        now,
    )
    .await;
    mailer
        .send(&Message {
            to: address.as_str().to_owned(),
            subject: "Reset your password".to_owned(),
            body: format!("Use this code to choose a new password: {}", secret),
        })
        .await;
}

pub async fn reset_password(
    users: &(impl UserRepository + ?Sized),
    tokens: &(impl MailTokenRepository + ?Sized),
    sessions: &(impl SessionRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    hasher: &(impl PasswordHasher + ?Sized),
    secret: &str,
    new: Password,
    now: OffsetDateTime,
) -> Result<User, RedeemError> {
    let token = tokens
        .find_by_digest(&digester.digest(secret))
        .await
        .filter(|t| t.purpose() == TokenPurpose::PasswordReset && t.is_redeemable_at(now))
        .ok_or(RedeemError::Unusable)?;
    let user = users
        .find_by_id(token.user_id())
        .await
        .ok_or(RedeemError::Unusable)?;
    let updated = user.with_password_hash(hasher.hash(new.as_str()));
    users.update(&updated).await;
    tokens.save(&token.redeemed(now)).await;
    sessions.delete_for_user(updated.id()).await;
    Ok(updated)
}

pub async fn request_email_change(
    users: &(impl UserRepository + ?Sized),
    tokens: &(impl MailTokenRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    mailer: &(impl Mailer + ?Sized),
    id: MailTokenId,
    user: &User,
    new_address: &Email,
    secret: String,
    now: OffsetDateTime,
) -> Result<(), ChangeEmailError> {
    if users.find_by_email(new_address).await.is_some() {
        return Err(ChangeEmailError::AddressTaken);
    }
    let secret = issue(
        tokens,
        digester,
        id,
        user.id(),
        TokenPurpose::EmailChange,
        secret,
        Some(new_address.as_str().to_owned()),
        now,
    )
    .await;
    mailer
        .send(&Message {
            to: new_address.as_str().to_owned(),
            subject: "Confirm your address".to_owned(),
            body: format!("Use this code to confirm this address: {}", secret),
        })
        .await;
    Ok(())
}

pub async fn confirm_email_change(
    users: &(impl UserRepository + ?Sized),
    tokens: &(impl MailTokenRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    secret: &str,
    now: OffsetDateTime,
) -> Result<User, RedeemError> {
    let token = tokens
        .find_by_digest(&digester.digest(secret))
        .await
        .filter(|t| t.purpose() == TokenPurpose::EmailChange && t.is_redeemable_at(now))
        .ok_or(RedeemError::Unusable)?;
    let address = token
        .payload()
        .and_then(|p| Email::parse(p).ok())
        .ok_or(RedeemError::Unusable)?;
    let user = users
        .find_by_id(token.user_id())
        .await
        .ok_or(RedeemError::Unusable)?;
    let updated = user.with_email(address).confirmed(now);
    users.update(&updated).await;
    tokens.save(&token.redeemed(now)).await;
    Ok(updated)
}

pub async fn request_activation(
    tokens: &(impl MailTokenRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    mailer: &(impl Mailer + ?Sized),
    id: MailTokenId,
    user: &User,
    secret: String,
    now: OffsetDateTime,
) {
    let secret = issue(
        tokens,
        digester,
        id,
        user.id(),
        TokenPurpose::Activation,
        secret,
        None,
        now,
    )
    .await;
    mailer
        .send(&Message {
            to: user.email().as_str().to_owned(),
            subject: "Confirm your address".to_owned(),
            body: format!("Use this code to confirm your address: {}", secret),
        })
        .await;
}

pub async fn confirm_activation(
    users: &(impl UserRepository + ?Sized),
    tokens: &(impl MailTokenRepository + ?Sized),
    digester: &(impl TokenDigest + ?Sized),
    secret: &str,
    now: OffsetDateTime,
) -> Result<User, RedeemError> {
    let token = tokens
        .find_by_digest(&digester.digest(secret))
        .await
        .filter(|t| t.purpose() == TokenPurpose::Activation && t.is_redeemable_at(now))
        .ok_or(RedeemError::Unusable)?;
    let user = users
        .find_by_id(token.user_id())
        .await
        .ok_or(RedeemError::Unusable)?;
    let updated = user.confirmed(now);
    users.update(&updated).await;
    tokens.save(&token.redeemed(now)).await;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        FakeHasher, FakeMailTokenRepo, FakeMailer, FakeSessionRepo, FakeUserRepo, PlainDigest,
    };
    use domain::{Session, SessionId, SessionToken, Username};
    use time::Duration;

    fn address() -> Email {
        Email::parse("owner@example.com").unwrap()
    }

    fn user() -> User {
        User::register(
            UserId::new(uuid::Uuid::nil()),
            Username::parse("mail_owner").unwrap(),
            address(),
            "hashed:correcthorse".to_owned(),
        )
    }

    fn token_id() -> MailTokenId {
        MailTokenId::new(uuid::Uuid::from_u128(7))
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH
    }

    struct Env {
        users: FakeUserRepo,
        tokens: FakeMailTokenRepo,
        sessions: FakeSessionRepo,
        mailer: FakeMailer,
    }

    fn env() -> Env {
        Env {
            users: FakeUserRepo::with(user()),
            tokens: FakeMailTokenRepo::new(),
            sessions: FakeSessionRepo::new(),
            mailer: FakeMailer::new(),
        }
    }

    async fn ask_for_reset(env: &Env, address: &Email) {
        request_password_reset(
            &env.users,
            &env.tokens,
            &PlainDigest,
            &env.mailer,
            token_id(),
            address,
            "secret-code".to_owned(),
            now(),
        )
        .await;
    }

    #[tokio::test]
    async fn a_reset_request_mails_a_code_to_the_address() {
        let env = env();
        ask_for_reset(&env, &address()).await;
        let sent = env.mailer.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "owner@example.com");
        assert_eq!(sent[0].subject, "Reset your password");
        assert!(sent[0].body.contains("secret-code"));
    }

    #[tokio::test]
    async fn an_unknown_address_is_answered_the_same_way_and_mails_nothing() {
        let env = env();
        let stranger = Email::parse("nobody@example.com").unwrap();
        ask_for_reset(&env, &stranger).await;
        assert!(env.mailer.sent().is_empty());
        assert!(env.tokens.all().is_empty());
    }

    #[tokio::test]
    async fn only_the_digest_of_the_code_is_stored() {
        let env = env();
        ask_for_reset(&env, &address()).await;
        let stored = env.tokens.all();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].digest(), "digest:secret-code");
        assert!(!stored[0].digest().contains("secret-code_plain"));
    }

    #[tokio::test]
    async fn the_code_sets_a_new_password_and_ends_every_session() {
        let env = env();
        env.sessions
            .save(&Session::new(
                SessionId::new(uuid::Uuid::nil()),
                user().id(),
                SessionToken::parse(&"t".repeat(32)).unwrap(),
                now() + Duration::days(1),
            ))
            .await;
        ask_for_reset(&env, &address()).await;
        let updated = reset_password(
            &env.users,
            &env.tokens,
            &env.sessions,
            &PlainDigest,
            &FakeHasher,
            "secret-code",
            Password::parse("brandnewpass").unwrap(),
            now(),
        )
        .await
        .unwrap();
        assert_eq!(updated.password_hash(), "hashed:brandnewpass");
        assert!(
            env.sessions
                .find_by_token(&SessionToken::parse(&"t".repeat(32)).unwrap())
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_code_cannot_be_used_twice() {
        let env = env();
        ask_for_reset(&env, &address()).await;
        let attempt = || {
            reset_password(
                &env.users,
                &env.tokens,
                &env.sessions,
                &PlainDigest,
                &FakeHasher,
                "secret-code",
                Password::parse("brandnewpass").unwrap(),
                now(),
            )
        };
        assert!(attempt().await.is_ok());
        assert_eq!(attempt().await, Err(RedeemError::Unusable));
    }

    #[tokio::test]
    async fn an_expired_code_is_refused() {
        let env = env();
        ask_for_reset(&env, &address()).await;
        let result = reset_password(
            &env.users,
            &env.tokens,
            &env.sessions,
            &PlainDigest,
            &FakeHasher,
            "secret-code",
            Password::parse("brandnewpass").unwrap(),
            now() + Duration::hours(2),
        )
        .await;
        assert_eq!(result, Err(RedeemError::Unusable));
    }

    #[tokio::test]
    async fn a_wrong_code_is_refused() {
        let env = env();
        ask_for_reset(&env, &address()).await;
        let result = reset_password(
            &env.users,
            &env.tokens,
            &env.sessions,
            &PlainDigest,
            &FakeHasher,
            "not-the-code",
            Password::parse("brandnewpass").unwrap(),
            now(),
        )
        .await;
        assert_eq!(result, Err(RedeemError::Unusable));
    }

    #[tokio::test]
    async fn an_activation_code_cannot_reset_a_password() {
        let env = env();
        request_activation(
            &env.tokens,
            &PlainDigest,
            &env.mailer,
            token_id(),
            &user(),
            "secret-code".to_owned(),
            now(),
        )
        .await;
        let result = reset_password(
            &env.users,
            &env.tokens,
            &env.sessions,
            &PlainDigest,
            &FakeHasher,
            "secret-code",
            Password::parse("brandnewpass").unwrap(),
            now(),
        )
        .await;
        assert_eq!(result, Err(RedeemError::Unusable));
    }

    #[tokio::test]
    async fn confirming_an_address_change_moves_the_account_to_it() {
        let env = env();
        let next = Email::parse("moved@example.com").unwrap();
        request_email_change(
            &env.users,
            &env.tokens,
            &PlainDigest,
            &env.mailer,
            token_id(),
            &user(),
            &next,
            "secret-code".to_owned(),
            now(),
        )
        .await
        .unwrap();
        assert_eq!(env.mailer.sent()[0].to, "moved@example.com");
        let updated =
            confirm_email_change(&env.users, &env.tokens, &PlainDigest, "secret-code", now())
                .await
                .unwrap();
        assert_eq!(updated.email(), &next);
        assert!(updated.is_confirmed());
    }

    #[tokio::test]
    async fn an_address_already_in_use_is_refused() {
        let env = env();
        let other = User::register(
            UserId::new(uuid::Uuid::max()),
            Username::parse("other_user").unwrap(),
            Email::parse("taken@example.com").unwrap(),
            "hash".to_owned(),
        );
        env.users.save(&other).await;
        let result = request_email_change(
            &env.users,
            &env.tokens,
            &PlainDigest,
            &env.mailer,
            token_id(),
            &user(),
            &Email::parse("taken@example.com").unwrap(),
            "secret-code".to_owned(),
            now(),
        )
        .await;
        assert_eq!(result, Err(ChangeEmailError::AddressTaken));
        assert!(env.mailer.sent().is_empty());
    }

    #[tokio::test]
    async fn activation_marks_the_address_confirmed() {
        let env = env();
        assert!(!user().is_confirmed());
        request_activation(
            &env.tokens,
            &PlainDigest,
            &env.mailer,
            token_id(),
            &user(),
            "secret-code".to_owned(),
            now(),
        )
        .await;
        let updated =
            confirm_activation(&env.users, &env.tokens, &PlainDigest, "secret-code", now())
                .await
                .unwrap();
        assert!(updated.is_confirmed());
    }
}
