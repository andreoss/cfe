use crate::paging::Paged;
use crate::ports::InvitationRepository;
use domain::{Invitation, InvitationCode, InvitationId, Page, User, UserId};
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvitationSettings {
    pub required: bool,
    pub lifetime: Duration,
    pub max_outstanding: u64,
}

impl Default for InvitationSettings {
    fn default() -> Self {
        Self {
            required: false,
            lifetime: Duration::days(7),
            max_outstanding: 5,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum IssueError {
    TooManyOutstanding,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpendError {
    Unknown,
    AlreadySpent,
    Expired,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdmissionError {
    CodeRequired,
    CodeNotUsable(SpendError),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Admission {
    invitation: Option<Invitation>,
}

impl Admission {
    pub fn invitation(&self) -> Option<&Invitation> {
        self.invitation.as_ref()
    }
}

pub async fn admit(
    invitations: &(impl InvitationRepository + ?Sized),
    settings: &InvitationSettings,
    code: Option<&InvitationCode>,
    now: OffsetDateTime,
) -> Result<Admission, AdmissionError> {
    let Some(code) = code else {
        if settings.required {
            return Err(AdmissionError::CodeRequired);
        }
        return Ok(Admission { invitation: None });
    };
    let invitation = invitations
        .find_by_code(code)
        .await
        .ok_or(AdmissionError::CodeNotUsable(SpendError::Unknown))?;
    if invitation.is_spent() {
        return Err(AdmissionError::CodeNotUsable(SpendError::AlreadySpent));
    }
    if !invitation.is_spendable_at(now) {
        return Err(AdmissionError::CodeNotUsable(SpendError::Expired));
    }
    Ok(Admission {
        invitation: Some(invitation),
    })
}

pub async fn issue_invitation(
    invitations: &(impl InvitationRepository + ?Sized),
    issuer: &User,
    settings: &InvitationSettings,
    id: InvitationId,
    code: InvitationCode,
    now: OffsetDateTime,
) -> Result<Invitation, IssueError> {
    let outstanding = invitations.count_outstanding(issuer.id(), now).await;
    if outstanding >= settings.max_outstanding {
        return Err(IssueError::TooManyOutstanding);
    }
    let invitation = Invitation::issue(id, code, issuer.id(), now, settings.lifetime);
    invitations.save(&invitation).await;
    Ok(invitation)
}

pub async fn spend_invitation(
    invitations: &(impl InvitationRepository + ?Sized),
    code: &InvitationCode,
    invitee: UserId,
    now: OffsetDateTime,
) -> Result<Invitation, SpendError> {
    let invitation = invitations
        .find_by_code(code)
        .await
        .ok_or(SpendError::Unknown)?;
    if invitation.is_spent() {
        return Err(SpendError::AlreadySpent);
    }
    if !invitation.is_spendable_at(now) {
        return Err(SpendError::Expired);
    }
    if !invitations.claim(invitation.id(), now).await {
        return Err(SpendError::AlreadySpent);
    }
    invitations.attribute(invitation.id(), invitee).await;
    Ok(invitation.spent(invitee, now))
}

pub async fn list_invitations(
    invitations: &(impl InvitationRepository + ?Sized),
    issuer: &User,
    page: Page,
) -> Paged<Invitation> {
    let items = invitations.list_by_issuer(issuer.id(), page).await;
    let total = invitations.count_by_issuer(issuer.id()).await;
    Paged::new(items, page, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeInvitationRepo;
    use domain::{Email, Username};

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn code(raw: &str) -> InvitationCode {
        InvitationCode::parse(raw).unwrap()
    }

    fn id(n: u128) -> InvitationId {
        InvitationId::new(uuid::Uuid::from_u128(n))
    }

    fn settings() -> InvitationSettings {
        InvitationSettings::default()
    }

    async fn issued(
        repo: &FakeInvitationRepo,
        issuer: &User,
        n: u128,
        raw: &str,
    ) -> Result<Invitation, IssueError> {
        issue_invitation(
            repo,
            issuer,
            &settings(),
            id(n),
            code(raw),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
    }

    #[tokio::test]
    async fn an_account_issues_a_code_someone_else_can_spend() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        let invitation = issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        assert_eq!(invitation.issuer_id(), issuer.id());
        let spent = spend_invitation(
            &repo,
            &code("ABCDEFGHJKLM"),
            UserId::new(uuid::Uuid::from_u128(2)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(
            spent.spent_by(),
            Some(UserId::new(uuid::Uuid::from_u128(2)))
        );
    }

    #[tokio::test]
    async fn a_code_is_spent_exactly_once() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        let first = UserId::new(uuid::Uuid::from_u128(2));
        let second = UserId::new(uuid::Uuid::from_u128(3));
        spend_invitation(
            &repo,
            &code("ABCDEFGHJKLM"),
            first,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(
            spend_invitation(
                &repo,
                &code("ABCDEFGHJKLM"),
                second,
                OffsetDateTime::UNIX_EPOCH
            )
            .await,
            Err(SpendError::AlreadySpent)
        );
    }

    #[tokio::test]
    async fn only_the_first_writer_claims_it() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        let invitation = issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        let first = UserId::new(uuid::Uuid::from_u128(2));
        assert!(
            repo.claim(invitation.id(), OffsetDateTime::UNIX_EPOCH)
                .await
        );
        assert!(
            !repo
                .claim(invitation.id(), OffsetDateTime::UNIX_EPOCH)
                .await
        );
        repo.attribute(invitation.id(), first).await;
        assert_eq!(
            repo.find_by_code(&code("ABCDEFGHJKLM"))
                .await
                .and_then(|i| i.spent_by()),
            Some(first)
        );
    }

    #[tokio::test]
    async fn a_released_claim_can_be_taken_by_someone_else() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        let invitation = issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        assert!(
            repo.claim(invitation.id(), OffsetDateTime::UNIX_EPOCH)
                .await
        );
        repo.release(invitation.id()).await;
        let later = UserId::new(uuid::Uuid::from_u128(3));
        let spent = spend_invitation(
            &repo,
            &code("ABCDEFGHJKLM"),
            later,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(spent.spent_by(), Some(later));
    }

    #[tokio::test]
    async fn a_code_nobody_issued_is_refused() {
        let repo = FakeInvitationRepo::new();
        assert_eq!(
            spend_invitation(
                &repo,
                &code("ZZZZZZZZZZZZ"),
                UserId::new(uuid::Uuid::from_u128(2)),
                OffsetDateTime::UNIX_EPOCH
            )
            .await,
            Err(SpendError::Unknown)
        );
    }

    #[tokio::test]
    async fn a_code_past_its_expiry_is_refused() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        assert_eq!(
            spend_invitation(
                &repo,
                &code("ABCDEFGHJKLM"),
                UserId::new(uuid::Uuid::from_u128(2)),
                OffsetDateTime::UNIX_EPOCH + Duration::days(7)
            )
            .await,
            Err(SpendError::Expired)
        );
    }

    #[tokio::test]
    async fn an_issuer_may_not_leave_more_than_the_settings_allow_outstanding() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        let codes = [
            "ABCDEFGHJKLM",
            "BBCDEFGHJKLM",
            "CBCDEFGHJKLM",
            "DBCDEFGHJKLM",
            "EBCDEFGHJKLM",
        ];
        for (n, raw) in codes.iter().enumerate() {
            issued(&repo, &issuer, n as u128 + 1, raw).await.unwrap();
        }
        assert_eq!(
            issued(&repo, &issuer, 6, "FBCDEFGHJKLM").await,
            Err(IssueError::TooManyOutstanding)
        );
    }

    #[tokio::test]
    async fn spending_one_makes_room_to_issue_another() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        let codes = [
            "ABCDEFGHJKLM",
            "BBCDEFGHJKLM",
            "CBCDEFGHJKLM",
            "DBCDEFGHJKLM",
            "EBCDEFGHJKLM",
        ];
        for (n, raw) in codes.iter().enumerate() {
            issued(&repo, &issuer, n as u128 + 1, raw).await.unwrap();
        }
        spend_invitation(
            &repo,
            &code("ABCDEFGHJKLM"),
            UserId::new(uuid::Uuid::from_u128(2)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(issued(&repo, &issuer, 6, "FBCDEFGHJKLM").await.is_ok());
    }

    #[tokio::test]
    async fn one_that_expired_stops_counting_against_the_issuer() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        let codes = [
            "ABCDEFGHJKLM",
            "BBCDEFGHJKLM",
            "CBCDEFGHJKLM",
            "DBCDEFGHJKLM",
            "EBCDEFGHJKLM",
        ];
        for (n, raw) in codes.iter().enumerate() {
            issued(&repo, &issuer, n as u128 + 1, raw).await.unwrap();
        }
        let later = OffsetDateTime::UNIX_EPOCH + Duration::days(8);
        assert!(
            issue_invitation(
                &repo,
                &issuer,
                &settings(),
                id(6),
                code("FBCDEFGHJKLM"),
                later
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn an_issuer_sees_only_the_codes_they_issued() {
        let repo = FakeInvitationRepo::new();
        let one = user(1, "issuer_01");
        let two = user(2, "issuer_02");
        issued(&repo, &one, 1, "ABCDEFGHJKLM").await.unwrap();
        issued(&repo, &two, 2, "BBCDEFGHJKLM").await.unwrap();
        let listed = list_invitations(&repo, &one, Page::first()).await;
        assert_eq!(listed.total, 1);
        assert_eq!(listed.items[0].code().as_str(), "ABCDEFGHJKLM");
    }

    #[tokio::test]
    async fn a_spent_code_stays_in_the_issuers_list() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        spend_invitation(
            &repo,
            &code("ABCDEFGHJKLM"),
            UserId::new(uuid::Uuid::from_u128(2)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let listed = list_invitations(&repo, &issuer, Page::first()).await;
        assert_eq!(listed.total, 1);
        assert!(listed.items[0].is_spent());
    }

    #[test]
    fn registration_is_open_unless_the_operator_closes_it() {
        assert!(!InvitationSettings::default().required);
    }

    fn closed() -> InvitationSettings {
        InvitationSettings {
            required: true,
            ..InvitationSettings::default()
        }
    }

    #[tokio::test]
    async fn nobody_needs_a_code_while_registration_is_open() {
        let repo = FakeInvitationRepo::new();
        let admitted = admit(&repo, &settings(), None, OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert!(admitted.invitation().is_none());
    }

    #[tokio::test]
    async fn a_code_is_demanded_once_the_operator_closes_registration() {
        let repo = FakeInvitationRepo::new();
        assert_eq!(
            admit(&repo, &closed(), None, OffsetDateTime::UNIX_EPOCH).await,
            Err(AdmissionError::CodeRequired)
        );
    }

    #[tokio::test]
    async fn a_valid_code_admits_whoever_holds_it() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        let admitted = admit(
            &repo,
            &closed(),
            Some(&code("ABCDEFGHJKLM")),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(
            admitted.invitation().map(|i| i.code().as_str()),
            Some("ABCDEFGHJKLM")
        );
    }

    #[tokio::test]
    async fn a_code_that_cannot_be_spent_admits_nobody() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        spend_invitation(
            &repo,
            &code("ABCDEFGHJKLM"),
            UserId::new(uuid::Uuid::from_u128(2)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(
            admit(
                &repo,
                &closed(),
                Some(&code("ABCDEFGHJKLM")),
                OffsetDateTime::UNIX_EPOCH
            )
            .await,
            Err(AdmissionError::CodeNotUsable(SpendError::AlreadySpent))
        );
        assert_eq!(
            admit(
                &repo,
                &closed(),
                Some(&code("ZZZZZZZZZZZZ")),
                OffsetDateTime::UNIX_EPOCH
            )
            .await,
            Err(AdmissionError::CodeNotUsable(SpendError::Unknown))
        );
    }

    #[tokio::test]
    async fn an_expired_code_admits_nobody() {
        let repo = FakeInvitationRepo::new();
        let issuer = user(1, "issuer_01");
        issued(&repo, &issuer, 1, "ABCDEFGHJKLM").await.unwrap();
        assert_eq!(
            admit(
                &repo,
                &closed(),
                Some(&code("ABCDEFGHJKLM")),
                OffsetDateTime::UNIX_EPOCH + Duration::days(7)
            )
            .await,
            Err(AdmissionError::CodeNotUsable(SpendError::Expired))
        );
    }

    #[tokio::test]
    async fn a_wrong_code_is_refused_even_while_registration_is_open() {
        let repo = FakeInvitationRepo::new();
        assert_eq!(
            admit(
                &repo,
                &settings(),
                Some(&code("ZZZZZZZZZZZZ")),
                OffsetDateTime::UNIX_EPOCH
            )
            .await,
            Err(AdmissionError::CodeNotUsable(SpendError::Unknown))
        );
    }
}
