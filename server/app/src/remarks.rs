use crate::paging::Paged;
use crate::ports::{RemarkRepository, UserRepository};
use domain::{Page, Remark, RemarkText, User, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum RemarkError {
    SubjectNotFound,
    NotYourself,
}

pub async fn set_remark(
    remarks: &(impl RemarkRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    author: &User,
    subject_id: UserId,
    text: RemarkText,
    now: OffsetDateTime,
) -> Result<Remark, RemarkError> {
    if author.id() == subject_id {
        return Err(RemarkError::NotYourself);
    }
    users
        .find_by_id(subject_id)
        .await
        .ok_or(RemarkError::SubjectNotFound)?;
    let remark = Remark::new(author.id(), subject_id, text, now);
    remarks.save(&remark).await;
    Ok(remark)
}

pub async fn clear_remark(
    remarks: &(impl RemarkRepository + ?Sized),
    author: &User,
    subject_id: UserId,
) {
    remarks.delete(author.id(), subject_id).await;
}

pub async fn remark_about(
    remarks: &(impl RemarkRepository + ?Sized),
    reader: &User,
    subject_id: UserId,
) -> Option<Remark> {
    remarks
        .find(reader.id(), subject_id)
        .await
        .filter(|r| r.readable_by(reader.id()))
}

pub async fn list_remarks(
    remarks: &(impl RemarkRepository + ?Sized),
    reader: &User,
    page: Page,
) -> Paged<Remark> {
    let items = remarks.list_by_author(reader.id(), page).await;
    let total = remarks.count_by_author(reader.id()).await;
    Paged::new(items, page, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeRemarkRepo, FakeUserRepo};
    use domain::{Email, Username};

    fn user(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn text(raw: &str) -> RemarkText {
        RemarkText::parse(raw).unwrap()
    }

    async fn seeded() -> (FakeRemarkRepo, FakeUserRepo, User, User) {
        let users = FakeUserRepo::new();
        let author = user(1, "author_01");
        let subject = user(2, "subject_01");
        users.save(&author).await;
        users.save(&subject).await;
        (FakeRemarkRepo::new(), users, author, subject)
    }

    #[tokio::test]
    async fn a_reader_writes_a_note_about_someone() {
        let (remarks, users, author, subject) = seeded().await;
        let remark = set_remark(
            &remarks,
            &users,
            &author,
            subject.id(),
            text("helpful"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(remark.text().as_str(), "helpful");
        assert_eq!(
            remark_about(&remarks, &author, subject.id())
                .await
                .map(|r| r.text().as_str().to_owned()),
            Some("helpful".to_owned())
        );
    }

    #[tokio::test]
    async fn writing_again_replaces_the_note() {
        let (remarks, users, author, subject) = seeded().await;
        for note in ["first", "second"] {
            set_remark(
                &remarks,
                &users,
                &author,
                subject.id(),
                text(note),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await
            .unwrap();
        }
        assert_eq!(
            remark_about(&remarks, &author, subject.id())
                .await
                .map(|r| r.text().as_str().to_owned()),
            Some("second".to_owned())
        );
        assert_eq!(list_remarks(&remarks, &author, Page::first()).await.total, 1);
    }

    #[tokio::test]
    async fn nobody_else_can_read_it() {
        let (remarks, users, author, subject) = seeded().await;
        set_remark(
            &remarks,
            &users,
            &author,
            subject.id(),
            text("private"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(remark_about(&remarks, &subject, subject.id()).await, None);
        let stranger = user(3, "stranger_01");
        assert_eq!(remark_about(&remarks, &stranger, subject.id()).await, None);
    }

    #[tokio::test]
    async fn a_note_about_yourself_is_refused() {
        let (remarks, users, author, _) = seeded().await;
        let result = set_remark(
            &remarks,
            &users,
            &author,
            author.id(),
            text("mine"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(RemarkError::NotYourself));
    }

    #[tokio::test]
    async fn a_note_about_an_unknown_account_is_refused() {
        let (remarks, users, author, _) = seeded().await;
        let result = set_remark(
            &remarks,
            &users,
            &author,
            UserId::new(uuid::Uuid::from_u128(404)),
            text("nobody"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(RemarkError::SubjectNotFound));
    }

    #[tokio::test]
    async fn a_note_can_be_cleared() {
        let (remarks, users, author, subject) = seeded().await;
        set_remark(
            &remarks,
            &users,
            &author,
            subject.id(),
            text("temporary"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        clear_remark(&remarks, &author, subject.id()).await;
        assert_eq!(remark_about(&remarks, &author, subject.id()).await, None);
        assert_eq!(list_remarks(&remarks, &author, Page::first()).await.total, 0);
    }

    #[tokio::test]
    async fn each_reader_keeps_their_own_notes() {
        let (remarks, users, author, subject) = seeded().await;
        let other = user(3, "other_01");
        users.save(&other).await;
        set_remark(
            &remarks,
            &users,
            &author,
            subject.id(),
            text("mine"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        set_remark(
            &remarks,
            &users,
            &other,
            subject.id(),
            text("theirs"),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(list_remarks(&remarks, &author, Page::first()).await.total, 1);
        assert_eq!(list_remarks(&remarks, &other, Page::first()).await.total, 1);
        assert_eq!(
            remark_about(&remarks, &other, subject.id())
                .await
                .map(|r| r.text().as_str().to_owned()),
            Some("theirs".to_owned())
        );
    }
}
