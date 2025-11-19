use crate::ports::{ReactionRepository, UserRepository};
use crate::reputation;
use domain::{Reaction, ReactionKind, ReactionTarget, UserId};
use time::OffsetDateTime;

pub struct ReactionSummary {
    pub counts: Vec<(ReactionKind, u64)>,
    pub mine: Option<ReactionKind>,
}

pub async fn react(
    reactions: &(impl ReactionRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    author_id: UserId,
    user_id: UserId,
    target: ReactionTarget,
    kind: ReactionKind,
    now: OffsetDateTime,
) {
    let previous = reactions.find_mine(user_id, target).await;
    reactions
        .save(&Reaction::new(user_id, target, kind, now))
        .await;
    reputation::apply_reaction(users, author_id, user_id, previous, Some(kind)).await;
}

pub async fn clear_reaction(
    reactions: &(impl ReactionRepository + ?Sized),
    users: &(impl UserRepository + ?Sized),
    author_id: UserId,
    user_id: UserId,
    target: ReactionTarget,
) {
    let previous = reactions.find_mine(user_id, target).await;
    reactions.delete(user_id, target).await;
    reputation::apply_reaction(users, author_id, user_id, previous, None).await;
}

pub async fn summarize_reactions(
    reactions: &(impl ReactionRepository + ?Sized),
    viewer_id: Option<UserId>,
    target: ReactionTarget,
) -> ReactionSummary {
    let counts = reactions.counts(target).await;
    let mine = match viewer_id {
        Some(id) => reactions.find_mine(id, target).await,
        None => None,
    };
    ReactionSummary { counts, mine }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeReactionRepo, FakeUserRepo};
    use domain::TopicId;

    fn user_id() -> UserId {
        UserId::new(uuid::Uuid::nil())
    }

    fn author_id() -> UserId {
        UserId::new(uuid::Uuid::from_u128(7))
    }

    fn other_id() -> UserId {
        UserId::new(uuid::Uuid::max())
    }

    fn target() -> ReactionTarget {
        ReactionTarget::Topic(TopicId::new(uuid::Uuid::nil()))
    }

    fn count_of(summary: &ReactionSummary, kind: ReactionKind) -> u64 {
        summary
            .counts
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    }

    #[tokio::test]
    async fn records_a_reaction_and_counts_it() {
        let repo = FakeReactionRepo::new();
        react(
            &repo,
            &FakeUserRepo::new(),
            author_id(),
            user_id(),
            target(),
            ReactionKind::Like,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        let summary = summarize_reactions(&repo, Some(user_id()), target()).await;
        assert_eq!(count_of(&summary, ReactionKind::Like), 1);
        assert_eq!(summary.mine, Some(ReactionKind::Like));
    }

    #[tokio::test]
    async fn reacting_again_replaces_the_previous_kind() {
        let repo = FakeReactionRepo::new();
        react(
            &repo,
            &FakeUserRepo::new(),
            author_id(),
            user_id(),
            target(),
            ReactionKind::Like,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        react(
            &repo,
            &FakeUserRepo::new(),
            author_id(),
            user_id(),
            target(),
            ReactionKind::Agree,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        let summary = summarize_reactions(&repo, Some(user_id()), target()).await;
        assert_eq!(count_of(&summary, ReactionKind::Like), 0);
        assert_eq!(count_of(&summary, ReactionKind::Agree), 1);
        assert_eq!(summary.mine, Some(ReactionKind::Agree));
    }

    #[tokio::test]
    async fn counts_each_users_reaction_once() {
        let repo = FakeReactionRepo::new();
        for id in [user_id(), other_id()] {
            react(
                &repo,
                &FakeUserRepo::new(),
                author_id(),
                id,
                target(),
                ReactionKind::Like,
                OffsetDateTime::UNIX_EPOCH,
            )
            .await;
        }
        let summary = summarize_reactions(&repo, Some(user_id()), target()).await;
        assert_eq!(count_of(&summary, ReactionKind::Like), 2);
    }

    #[tokio::test]
    async fn clearing_removes_only_your_own_reaction() {
        let repo = FakeReactionRepo::new();
        for id in [user_id(), other_id()] {
            react(
                &repo,
                &FakeUserRepo::new(),
                author_id(),
                id,
                target(),
                ReactionKind::Like,
                OffsetDateTime::UNIX_EPOCH,
            )
            .await;
        }
        clear_reaction(&repo, &FakeUserRepo::new(), author_id(), user_id(), target()).await;
        let summary = summarize_reactions(&repo, Some(user_id()), target()).await;
        assert_eq!(count_of(&summary, ReactionKind::Like), 1);
        assert_eq!(summary.mine, None);
    }

    #[tokio::test]
    async fn an_anonymous_viewer_sees_counts_but_no_own_reaction() {
        let repo = FakeReactionRepo::new();
        react(
            &repo,
            &FakeUserRepo::new(),
            author_id(),
            user_id(),
            target(),
            ReactionKind::Like,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        let summary = summarize_reactions(&repo, None, target()).await;
        assert_eq!(count_of(&summary, ReactionKind::Like), 1);
        assert_eq!(summary.mine, None);
    }

    #[tokio::test]
    async fn reactions_on_one_target_do_not_affect_another() {
        let repo = FakeReactionRepo::new();
        let other_target = ReactionTarget::Topic(TopicId::new(uuid::Uuid::max()));
        react(
            &repo,
            &FakeUserRepo::new(),
            author_id(),
            user_id(),
            target(),
            ReactionKind::Like,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await;
        let summary = summarize_reactions(&repo, Some(user_id()), other_target).await;
        assert_eq!(count_of(&summary, ReactionKind::Like), 0);
        assert_eq!(summary.mine, None);
    }
}
