use crate::ports::UserRepository;
use domain::{Penalty, ReactionKind, UserId, for_reaction};

async fn adjust(users: &(impl UserRepository + ?Sized), author_id: UserId, delta: i32) {
    if delta == 0 {
        return;
    }
    let Some(author) = users.find_by_id(author_id).await else {
        return;
    };
    let adjusted = author.with_score(author.score().changed(delta));
    users.update(&adjusted).await;
}

pub async fn apply_reaction(
    users: &(impl UserRepository + ?Sized),
    author_id: UserId,
    actor_id: UserId,
    previous: Option<ReactionKind>,
    next: Option<ReactionKind>,
) {
    if author_id == actor_id {
        return;
    }
    let removed = previous.map(for_reaction).unwrap_or(0);
    let added = next.map(for_reaction).unwrap_or(0);
    adjust(users, author_id, added - removed).await;
}

pub async fn give_back(
    users: &(impl UserRepository + ?Sized),
    author_id: UserId,
    penalty: Penalty,
) {
    adjust(users, author_id, -penalty.value()).await;
}

pub async fn apply_deletion(
    users: &(impl UserRepository + ?Sized),
    author_id: UserId,
    penalty: Penalty,
) {
    adjust(users, author_id, penalty.value()).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeUserRepo;
    use domain::{Email, Penalty, Score, User, Username, for_deletion};
    use time::OffsetDateTime;

    fn author_id() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn actor_id() -> UserId {
        UserId::new(uuid::Uuid::max())
    }

    async fn repo_with_author() -> FakeUserRepo {
        let users = FakeUserRepo::new();
        users
            .save(&User::from_parts(
                author_id(),
                Username::parse("author").expect("a username"),
                Email::parse("author@example.com").expect("an address"),
                "hash".to_string(),
                None,
                domain::Role::User,
                None,
                None,
                Score::initial(),
                OffsetDateTime::UNIX_EPOCH,
            ))
            .await;
        users
    }

    async fn score_of(users: &FakeUserRepo) -> i32 {
        users
            .find_by_id(author_id())
            .await
            .expect("the author")
            .score()
            .value()
    }

    #[tokio::test]
    async fn a_first_approval_earns() {
        let users = repo_with_author().await;
        apply_reaction(
            &users,
            author_id(),
            actor_id(),
            None,
            Some(ReactionKind::Like),
        )
        .await;
        assert_eq!(score_of(&users).await, 1);
    }

    #[tokio::test]
    async fn switching_a_reaction_reverses_the_previous_one() {
        let users = repo_with_author().await;
        apply_reaction(
            &users,
            author_id(),
            actor_id(),
            None,
            Some(ReactionKind::Thanks),
        )
        .await;
        apply_reaction(
            &users,
            author_id(),
            actor_id(),
            Some(ReactionKind::Thanks),
            Some(ReactionKind::Disagree),
        )
        .await;
        assert_eq!(score_of(&users).await, -1);
    }

    #[tokio::test]
    async fn clearing_a_reaction_returns_the_score() {
        let users = repo_with_author().await;
        apply_reaction(
            &users,
            author_id(),
            actor_id(),
            None,
            Some(ReactionKind::Agree),
        )
        .await;
        apply_reaction(
            &users,
            author_id(),
            actor_id(),
            Some(ReactionKind::Agree),
            None,
        )
        .await;
        assert_eq!(score_of(&users).await, 0);
    }

    #[tokio::test]
    async fn reacting_to_your_own_post_changes_nothing() {
        let users = repo_with_author().await;
        apply_reaction(
            &users,
            author_id(),
            author_id(),
            None,
            Some(ReactionKind::Thanks),
        )
        .await;
        assert_eq!(score_of(&users).await, 0);
    }

    #[tokio::test]
    async fn a_deletion_costs_the_author() {
        let users = repo_with_author().await;
        apply_deletion(&users, author_id(), Penalty::default()).await;
        assert_eq!(score_of(&users).await, for_deletion());
    }

    #[tokio::test]
    async fn an_unknown_author_is_ignored() {
        let users = repo_with_author().await;
        apply_deletion(&users, actor_id(), Penalty::default()).await;
        assert_eq!(score_of(&users).await, 0);
    }

    #[tokio::test]
    async fn the_floor_holds_under_repeated_deletions() {
        let users = repo_with_author().await;
        for _ in 0..100 {
            apply_deletion(&users, author_id(), Penalty::default()).await;
        }
        assert_eq!(score_of(&users).await, domain::SCORE_MIN);
    }
}
