use crate::paging::Paged;
use crate::ports::{CommentRepository, ReportRepository, TopicRepository, UserRepository};
use domain::{
    CommentId, Page, Reason, Report, ReportId, ReportKind, ReportTarget, TopicId, User, UserId,
};
use time::{Duration, OffsetDateTime};

pub const REPORTS_PER_HOUR: u64 = 10;
pub const REPORT_WINDOW: Duration = Duration::hours(1);

#[derive(Debug, PartialEq, Eq)]
pub enum ReportError {
    TargetNotFound,
    AlreadyReported,
    TooMany,
    NotAuthorized,
    NotFound,
    AlreadyClosed,
}

pub async fn report_content(
    reports: &(impl ReportRepository + ?Sized),
    topics: &(impl TopicRepository + ?Sized),
    comments: &(impl CommentRepository + ?Sized),
    id: ReportId,
    reporter: &User,
    topic_id: TopicId,
    comment_id: Option<CommentId>,
    kind: ReportKind,
    reason: Reason,
    now: OffsetDateTime,
) -> Result<Report, ReportError> {
    let target = match comment_id {
        Some(comment_id) => {
            let comment = comments
                .find_by_id(comment_id)
                .await
                .ok_or(ReportError::TargetNotFound)?;
            if comment.topic_id() != topic_id {
                return Err(ReportError::TargetNotFound);
            }
            ReportTarget::Comment(topic_id, comment_id)
        }
        None => {
            topics
                .find_by_id(topic_id)
                .await
                .ok_or(ReportError::TargetNotFound)?;
            ReportTarget::Topic(topic_id)
        }
    };
    if reports
        .find_open_by_reporter(reporter.id(), target)
        .await
        .is_some()
    {
        return Err(ReportError::AlreadyReported);
    }
    let since = now - REPORT_WINDOW;
    if reports.count_by_reporter_since(reporter.id(), since).await >= REPORTS_PER_HOUR {
        return Err(ReportError::TooMany);
    }
    let report = Report::open(id, target, reporter.id(), kind, reason, now);
    reports.save(&report).await;
    Ok(report)
}

pub async fn list_open_reports(
    reports: &(impl ReportRepository + ?Sized),
    moderator: &User,
    page: Page,
) -> Result<Paged<Report>, ReportError> {
    if !moderator.role().is_moderator() {
        return Err(ReportError::NotAuthorized);
    }
    let items = reports.list_open(page).await;
    let total = reports.count_open().await;
    Ok(Paged::new(items, page, total))
}

pub async fn close_report(
    reports: &(impl ReportRepository + ?Sized),
    moderator: &User,
    id: ReportId,
    now: OffsetDateTime,
) -> Result<Report, ReportError> {
    if !moderator.role().is_moderator() {
        return Err(ReportError::NotAuthorized);
    }
    let report = reports.find_by_id(id).await.ok_or(ReportError::NotFound)?;
    if !report.is_open() {
        return Err(ReportError::AlreadyClosed);
    }
    let closed = report.closed(moderator.id(), now);
    reports.update(&closed).await;
    Ok(closed)
}

pub async fn count_open_for_topic(
    reports: &(impl ReportRepository + ?Sized),
    topic_id: TopicId,
) -> u64 {
    reports.count_open_for_topic(topic_id).await
}

pub async fn reporter_of(
    users: &(impl UserRepository + ?Sized),
    reporter_id: UserId,
) -> Option<User> {
    users.find_by_id(reporter_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeCommentRepo, FakeReportRepo, FakeTopicRepo};
    use domain::{Body, Comment, Email, SectionId, TagSet, Title, Topic, Username};

    fn topic_id() -> TopicId {
        TopicId::new(uuid::Uuid::nil())
    }

    fn comment_id() -> CommentId {
        CommentId::new(uuid::Uuid::max())
    }

    fn plain(id: u128, name: &str) -> User {
        User::register(
            UserId::new(uuid::Uuid::from_u128(id)),
            Username::parse(name).unwrap(),
            Email::parse(&format!("{name}@example.com")).unwrap(),
            "hash".to_owned(),
        )
    }

    fn moderator() -> User {
        plain(99, "keeper_01").promoted_to_moderator()
    }

    fn topic() -> Topic {
        Topic::new(
            topic_id(),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::from_u128(5)),
            Title::parse("Hello").unwrap(),
            Body::parse("World").unwrap(),
            TagSet::empty(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn comment() -> Comment {
        Comment::new(
            comment_id(),
            topic_id(),
            UserId::new(uuid::Uuid::from_u128(5)),
            None,
            Body::parse("A comment").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    fn reason() -> Reason {
        Reason::parse("breaks the rules").unwrap()
    }

    fn report_id(n: u128) -> ReportId {
        ReportId::new(uuid::Uuid::from_u128(n))
    }

    async fn env() -> (FakeReportRepo, FakeTopicRepo, FakeCommentRepo) {
        let comments = FakeCommentRepo::new();
        comments.save(&comment()).await;
        (FakeReportRepo::new(), FakeTopicRepo::with(topic()), comments)
    }

    #[tokio::test]
    async fn a_reader_reports_a_topic() {
        let (reports, topics, comments) = env().await;
        let report = report_content(
            &reports,
            &topics,
            &comments,
            report_id(1),
            &plain(1, "reader_01"),
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert!(report.is_open());
        assert_eq!(report.target(), ReportTarget::Topic(topic_id()));
        assert_eq!(reports.count_open().await, 1);
    }

    #[tokio::test]
    async fn a_reader_reports_a_comment() {
        let (reports, topics, comments) = env().await;
        let report = report_content(
            &reports,
            &topics,
            &comments,
            report_id(2),
            &plain(1, "reader_01"),
            topic_id(),
            Some(comment_id()),
            ReportKind::Spelling,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        assert_eq!(
            report.target(),
            ReportTarget::Comment(topic_id(), comment_id())
        );
    }

    #[tokio::test]
    async fn reporting_an_unknown_target_is_refused() {
        let (reports, topics, comments) = env().await;
        let result = report_content(
            &reports,
            &topics,
            &comments,
            report_id(3),
            &plain(1, "reader_01"),
            TopicId::new(uuid::Uuid::from_u128(404)),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(ReportError::TargetNotFound));
    }

    #[tokio::test]
    async fn reporting_the_same_post_twice_is_refused_while_it_is_open() {
        let (reports, topics, comments) = env().await;
        let reader = plain(1, "reader_01");
        let first = report_content(
            &reports,
            &topics,
            &comments,
            report_id(4),
            &reader,
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert!(first.is_ok());
        let second = report_content(
            &reports,
            &topics,
            &comments,
            report_id(5),
            &reader,
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(second, Err(ReportError::AlreadyReported));
    }

    #[tokio::test]
    async fn another_reader_may_report_the_same_post() {
        let (reports, topics, comments) = env().await;
        for (n, name) in [(6u128, "reader_01"), (7, "reader_02")] {
            let result = report_content(
                &reports,
                &topics,
                &comments,
                report_id(n),
                &plain(n, name),
                topic_id(),
                None,
                ReportKind::Rule,
                reason(),
                OffsetDateTime::UNIX_EPOCH,
            )
            .await;
            assert!(result.is_ok());
        }
        assert_eq!(reports.count_open_for_topic(topic_id()).await, 2);
    }

    #[tokio::test]
    async fn a_reader_may_not_flood_reports() {
        let (reports, topics, comments) = env().await;
        let reader = plain(1, "reader_01");
        reports.set_recent_count(reader.id(), REPORTS_PER_HOUR);
        let result = report_content(
            &reports,
            &topics,
            &comments,
            report_id(8),
            &reader,
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(ReportError::TooMany));
    }

    #[tokio::test]
    async fn only_a_moderator_sees_the_queue() {
        let (reports, _, _) = env().await;
        let refused = list_open_reports(&reports, &plain(1, "reader_01"), Page::first()).await;
        assert_eq!(refused.err(), Some(ReportError::NotAuthorized));
        let allowed = list_open_reports(&reports, &moderator(), Page::first()).await;
        assert!(allowed.is_ok());
    }

    #[tokio::test]
    async fn closing_a_report_takes_it_out_of_the_queue() {
        let (reports, topics, comments) = env().await;
        let report = report_content(
            &reports,
            &topics,
            &comments,
            report_id(9),
            &plain(1, "reader_01"),
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let keeper = moderator();
        let closed = close_report(&reports, &keeper, report.id(), OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert!(!closed.is_open());
        assert_eq!(closed.closed_by(), Some(keeper.id()));
        assert_eq!(reports.count_open().await, 0);
        assert_eq!(reports.count_open_for_topic(topic_id()).await, 0);
    }

    #[tokio::test]
    async fn closing_a_closed_report_is_refused() {
        let (reports, topics, comments) = env().await;
        let report = report_content(
            &reports,
            &topics,
            &comments,
            report_id(10),
            &plain(1, "reader_01"),
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let keeper = moderator();
        close_report(&reports, &keeper, report.id(), OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        let again = close_report(&reports, &keeper, report.id(), OffsetDateTime::UNIX_EPOCH).await;
        assert_eq!(again, Err(ReportError::AlreadyClosed));
    }

    #[tokio::test]
    async fn a_plain_user_cannot_close_a_report() {
        let (reports, topics, comments) = env().await;
        let report = report_content(
            &reports,
            &topics,
            &comments,
            report_id(11),
            &plain(1, "reader_01"),
            topic_id(),
            None,
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
        let result = close_report(
            &reports,
            &plain(2, "reader_02"),
            report.id(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(ReportError::NotAuthorized));
    }

    #[tokio::test]
    async fn closing_an_unknown_report_is_refused() {
        let (reports, _, _) = env().await;
        let result = close_report(
            &reports,
            &moderator(),
            report_id(404),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(ReportError::NotFound));
    }

    #[tokio::test]
    async fn reporting_a_comment_from_another_topic_is_refused() {
        let (reports, topics, comments) = env().await;
        let result = report_content(
            &reports,
            &topics,
            &comments,
            report_id(12),
            &plain(1, "reader_01"),
            TopicId::new(uuid::Uuid::from_u128(77)),
            Some(comment_id()),
            ReportKind::Rule,
            reason(),
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        assert_eq!(result, Err(ReportError::TargetNotFound));
    }
}
