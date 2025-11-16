use crate::ports::NotificationRepository;
use crate::paging::Paged;
use domain::{Notification, NotificationId, Page, UserId};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq)]
pub enum MarkReadError {
    NotFound,
}

pub async fn list_notifications(
    repo: &(impl NotificationRepository + ?Sized),
    recipient_id: UserId,
    page: Page,
) -> Paged<Notification> {
    let items = repo.list_by_recipient(recipient_id, page).await;
    let total = repo.count_by_recipient(recipient_id).await;
    Paged::new(items, page, total)
}

pub async fn count_unread(repo: &(impl NotificationRepository + ?Sized), recipient_id: UserId) -> u64 {
    repo.count_unread(recipient_id).await
}

pub async fn mark_read(
    repo: &(impl NotificationRepository + ?Sized),
    recipient_id: UserId,
    id: NotificationId,
    now: OffsetDateTime,
) -> Result<Notification, MarkReadError> {
    let notification = repo.find_by_id(id).await.ok_or(MarkReadError::NotFound)?;
    if notification.recipient_id() != recipient_id {
        return Err(MarkReadError::NotFound);
    }
    let read = notification.marked_read(now);
    repo.update(&read).await;
    Ok(read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeNotificationRepo;
    use domain::{CommentId, TopicId};

    fn recipient() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn notification(id: uuid::Uuid, owner: UserId) -> Notification {
        Notification::new(
            NotificationId::new(id),
            owner,
            UserId::new(uuid::Uuid::max()),
            TopicId::new(uuid::Uuid::nil()),
            CommentId::new(uuid::Uuid::nil()),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[tokio::test]
    async fn lists_only_the_recipients_own_notifications() {
        let repo = FakeNotificationRepo::new();
        repo.save(&notification(uuid::Uuid::from_u128(1), recipient()))
            .await;
        repo.save(&notification(
            uuid::Uuid::from_u128(2),
            UserId::new(uuid::Uuid::max()),
        ))
        .await;
        let listed = list_notifications(&repo, recipient(), Page::first()).await;
        assert_eq!(listed.items.len(), 1);
        assert_eq!(listed.items[0].recipient_id(), recipient());
    }

    #[tokio::test]
    async fn counts_only_unread_notifications() {
        let repo = FakeNotificationRepo::new();
        repo.save(&notification(uuid::Uuid::from_u128(1), recipient()))
            .await;
        let read = notification(uuid::Uuid::from_u128(2), recipient())
            .marked_read(OffsetDateTime::UNIX_EPOCH);
        repo.save(&read).await;
        assert_eq!(count_unread(&repo, recipient()).await, 1);
    }

    #[tokio::test]
    async fn marks_an_own_notification_read() {
        let repo = FakeNotificationRepo::new();
        let n = notification(uuid::Uuid::from_u128(1), recipient());
        repo.save(&n).await;
        let marked = mark_read(&repo, recipient(), n.id(), OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert!(marked.is_read());
        assert_eq!(count_unread(&repo, recipient()).await, 0);
    }

    #[tokio::test]
    async fn refuses_to_mark_someone_elses_notification_read() {
        let repo = FakeNotificationRepo::new();
        let n = notification(uuid::Uuid::from_u128(1), UserId::new(uuid::Uuid::max()));
        repo.save(&n).await;
        let result = mark_read(&repo, recipient(), n.id(), OffsetDateTime::UNIX_EPOCH).await;
        assert_eq!(result, Err(MarkReadError::NotFound));
    }

    #[tokio::test]
    async fn refuses_to_mark_an_unknown_notification_read() {
        let repo = FakeNotificationRepo::new();
        let result = mark_read(
            &repo,
            recipient(),
            NotificationId::new(uuid::Uuid::from_u128(9)),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(MarkReadError::NotFound));
    }
}
