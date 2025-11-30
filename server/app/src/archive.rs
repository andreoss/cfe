use crate::paging::Paged;
use crate::ports::TopicRepository;
use crate::topics::Visibility;
use domain::{Page, Topic};
use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArchiveMonth {
    pub year: i32,
    pub month: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MonthError;

impl ArchiveMonth {
    pub fn new(year: i32, month: u8) -> Result<Self, MonthError> {
        if !(1..=12).contains(&month) {
            return Err(MonthError);
        }
        if !(1970..=9999).contains(&year) {
            return Err(MonthError);
        }
        Ok(Self { year, month })
    }

    pub fn of(at: OffsetDateTime) -> Self {
        Self {
            year: at.year(),
            month: at.month() as u8,
        }
    }

    pub fn starts_at(&self) -> OffsetDateTime {
        let month = Month::try_from(self.month).expect("month was checked");
        let date = Date::from_calendar_date(self.year, month, 1).expect("first of a month exists");
        OffsetDateTime::new_in_offset(date, Time::MIDNIGHT, UtcOffset::UTC)
    }

    pub fn next(&self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    pub fn ends_before(&self) -> OffsetDateTime {
        self.next().starts_at()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthCount {
    pub month: ArchiveMonth,
    pub topics: u64,
}

pub async fn months_with_topics(
    topics: &(impl TopicRepository + ?Sized),
    visibility: Visibility,
) -> Vec<MonthCount> {
    let mut counted: Vec<MonthCount> = Vec::new();
    for topic in topics.all_for_archive().await {
        if !visibility.allows(&topic) {
            continue;
        }
        let month = ArchiveMonth::of(topic.created_at());
        match counted.iter_mut().find(|c| c.month == month) {
            Some(entry) => entry.topics += 1,
            None => counted.push(MonthCount { month, topics: 1 }),
        }
    }
    counted.sort_by(|a, b| b.month.cmp(&a.month));
    counted
}

pub async fn topics_in_month(
    topics: &(impl TopicRepository + ?Sized),
    month: ArchiveMonth,
    page: Page,
    visibility: Visibility,
) -> Paged<Topic> {
    let found = topics
        .list_between(month.starts_at(), month.ends_before(), page)
        .await;
    let visible: Vec<Topic> = found.into_iter().filter(|t| visibility.allows(t)).collect();
    let total = topics
        .count_between(month.starts_at(), month.ends_before())
        .await;
    Paged::new(visible, page, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeTopicRepo;
    use domain::{Body, SectionId, TagSet, Title, TopicId, UserId};
    use time::Duration;

    fn topic(n: u128, at: OffsetDateTime) -> Topic {
        Topic::new(
            TopicId::new(uuid::Uuid::from_u128(n)),
            SectionId::new(uuid::Uuid::nil()),
            UserId::new(uuid::Uuid::from_u128(1)),
            Title::parse(&format!("Subject {n}")).unwrap(),
            Body::parse("A body.").unwrap(),
            TagSet::parse(&[]).unwrap(),
            at,
        )
        .with_pending(false)
    }

    fn at(year: i32, month: u8, day: u8) -> OffsetDateTime {
        OffsetDateTime::new_in_offset(
            Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap(),
            Time::MIDNIGHT,
            UtcOffset::UTC,
        )
    }

    #[test]
    fn a_month_is_within_the_calendar() {
        assert!(ArchiveMonth::new(2024, 1).is_ok());
        assert!(ArchiveMonth::new(2024, 12).is_ok());
        assert_eq!(ArchiveMonth::new(2024, 0), Err(MonthError));
        assert_eq!(ArchiveMonth::new(2024, 13), Err(MonthError));
        assert_eq!(ArchiveMonth::new(1969, 6), Err(MonthError));
    }

    #[test]
    fn a_month_spans_from_its_first_day_to_the_next() {
        let june = ArchiveMonth::new(2024, 6).unwrap();
        assert_eq!(june.starts_at(), at(2024, 6, 1));
        assert_eq!(june.ends_before(), at(2024, 7, 1));
    }

    #[test]
    fn december_rolls_into_the_next_year() {
        let december = ArchiveMonth::new(2024, 12).unwrap();
        assert_eq!(december.next(), ArchiveMonth::new(2025, 1).unwrap());
        assert_eq!(december.ends_before(), at(2025, 1, 1));
    }

    #[test]
    fn a_month_is_read_off_a_moment() {
        assert_eq!(
            ArchiveMonth::of(at(2024, 6, 17)),
            ArchiveMonth::new(2024, 6).unwrap()
        );
    }

    #[tokio::test]
    async fn months_are_listed_newest_first_with_their_counts() {
        let repo = FakeTopicRepo::new();
        repo.save(&topic(1, at(2024, 6, 2))).await;
        repo.save(&topic(2, at(2024, 6, 20))).await;
        repo.save(&topic(3, at(2024, 5, 9))).await;
        let months = months_with_topics(&repo, Visibility::moderator()).await;
        assert_eq!(
            months,
            vec![
                MonthCount {
                    month: ArchiveMonth::new(2024, 6).unwrap(),
                    topics: 2
                },
                MonthCount {
                    month: ArchiveMonth::new(2024, 5).unwrap(),
                    topics: 1
                },
            ]
        );
    }

    #[tokio::test]
    async fn a_month_nobody_posted_in_is_not_listed() {
        let repo = FakeTopicRepo::new();
        repo.save(&topic(1, at(2024, 6, 2))).await;
        let months = months_with_topics(&repo, Visibility::moderator()).await;
        assert_eq!(months.len(), 1);
    }

    #[tokio::test]
    async fn a_queued_subject_is_counted_only_for_those_who_may_see_it() {
        let repo = FakeTopicRepo::new();
        repo.save(&topic(1, at(2024, 6, 2))).await;
        repo.save(&topic(2, at(2024, 6, 3)).with_pending(true))
            .await;
        let seen_by_anyone = months_with_topics(&repo, Visibility::anonymous()).await;
        assert_eq!(seen_by_anyone[0].topics, 1);
        let seen_by_moderator = months_with_topics(&repo, Visibility::moderator()).await;
        assert_eq!(seen_by_moderator[0].topics, 2);
    }

    #[tokio::test]
    async fn a_month_lists_only_what_was_posted_in_it() {
        let repo = FakeTopicRepo::new();
        repo.save(&topic(1, at(2024, 6, 1))).await;
        repo.save(&topic(2, at(2024, 6, 30) + Duration::hours(23)))
            .await;
        repo.save(&topic(3, at(2024, 7, 1))).await;
        repo.save(&topic(4, at(2024, 5, 31))).await;
        let june = topics_in_month(
            &repo,
            ArchiveMonth::new(2024, 6).unwrap(),
            Page::first(),
            Visibility::moderator(),
        )
        .await;
        assert_eq!(june.items.len(), 2);
        assert_eq!(june.total, 2);
    }

    #[tokio::test]
    async fn the_first_moment_of_a_month_belongs_to_it_and_the_last_does_not() {
        let repo = FakeTopicRepo::new();
        repo.save(&topic(1, at(2024, 6, 1))).await;
        repo.save(&topic(2, at(2024, 7, 1))).await;
        let june = topics_in_month(
            &repo,
            ArchiveMonth::new(2024, 6).unwrap(),
            Page::first(),
            Visibility::moderator(),
        )
        .await;
        assert_eq!(june.items.len(), 1);
        assert_eq!(june.items[0].id(), TopicId::new(uuid::Uuid::from_u128(1)));
    }

    #[tokio::test]
    async fn a_queued_subject_stays_out_of_a_month_for_a_stranger() {
        let repo = FakeTopicRepo::new();
        repo.save(&topic(1, at(2024, 6, 2))).await;
        repo.save(&topic(2, at(2024, 6, 3)).with_pending(true))
            .await;
        let seen = topics_in_month(
            &repo,
            ArchiveMonth::new(2024, 6).unwrap(),
            Page::first(),
            Visibility::anonymous(),
        )
        .await;
        assert_eq!(seen.items.len(), 1);
    }

    #[tokio::test]
    async fn a_month_with_nothing_in_it_is_empty_rather_than_an_error() {
        let repo = FakeTopicRepo::new();
        let empty = topics_in_month(
            &repo,
            ArchiveMonth::new(2024, 6).unwrap(),
            Page::first(),
            Visibility::moderator(),
        )
        .await;
        assert!(empty.items.is_empty());
        assert_eq!(empty.total, 0);
    }
}
