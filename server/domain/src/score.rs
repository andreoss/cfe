use crate::ReactionKind;

pub const MIN: i32 = -50;
pub const MAX: i32 = 100_000;

pub const DELETION_PENALTY: i32 = -10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(i32);

impl Score {
    pub fn initial() -> Self {
        Self(0)
    }

    pub fn of(value: i32) -> Self {
        Self(value.clamp(MIN, MAX))
    }

    pub fn value(&self) -> i32 {
        self.0
    }

    pub fn changed(&self, delta: i32) -> Self {
        Self::of(self.0.saturating_add(delta))
    }
}

impl Default for Score {
    fn default() -> Self {
        Self::initial()
    }
}

pub fn for_reaction(kind: ReactionKind) -> i32 {
    match kind {
        ReactionKind::Like | ReactionKind::Agree => 1,
        ReactionKind::Thanks => 2,
        ReactionKind::Disagree => -1,
    }
}

pub const MAX_DELETION_PENALTY: i32 = -50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Penalty(i32);

#[derive(Debug, PartialEq, Eq)]
pub struct PenaltyError;

impl Penalty {
    pub fn parse(value: i32) -> Result<Self, PenaltyError> {
        if (MAX_DELETION_PENALTY..=0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(PenaltyError)
        }
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

impl Default for Penalty {
    fn default() -> Self {
        Self(DELETION_PENALTY)
    }
}

pub fn for_deletion() -> i32 {
    DELETION_PENALTY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_penalty_the_moderator_chooses_stays_within_bounds() {
        assert_eq!(Penalty::parse(0).unwrap().value(), 0);
        assert_eq!(Penalty::parse(-10).unwrap().value(), -10);
        assert_eq!(
            Penalty::parse(MAX_DELETION_PENALTY).unwrap().value(),
            MAX_DELETION_PENALTY
        );
    }

    #[test]
    fn a_penalty_outside_the_bounds_is_refused() {
        assert_eq!(Penalty::parse(1), Err(PenaltyError));
        assert_eq!(Penalty::parse(MAX_DELETION_PENALTY - 1), Err(PenaltyError));
        assert_eq!(Penalty::parse(-1000), Err(PenaltyError));
    }

    #[test]
    fn the_penalty_left_unchosen_is_the_one_that_was_always_applied() {
        assert_eq!(Penalty::default().value(), DELETION_PENALTY);
        assert_eq!(Penalty::default().value(), for_deletion());
    }

    #[test]
    fn a_penalty_of_nothing_leaves_a_score_alone() {
        let score = Score::of(20);
        assert_eq!(score.changed(Penalty::parse(0).unwrap().value()), score);
    }

    #[test]
    fn a_new_account_starts_at_zero() {
        assert_eq!(Score::initial().value(), 0);
        assert_eq!(Score::default(), Score::initial());
    }

    #[test]
    fn a_change_adds_the_delta() {
        assert_eq!(Score::initial().changed(3).value(), 3);
        assert_eq!(Score::of(10).changed(-4).value(), 6);
        assert_eq!(Score::of(10).changed(0).value(), 10);
    }

    #[test]
    fn a_score_never_falls_below_the_floor() {
        assert_eq!(Score::of(MIN).changed(-1).value(), MIN);
        assert_eq!(Score::of(-1_000_000).value(), MIN);
        assert_eq!(Score::initial().changed(i32::MIN).value(), MIN);
    }

    #[test]
    fn a_score_never_rises_above_the_ceiling() {
        assert_eq!(Score::of(MAX).changed(1).value(), MAX);
        assert_eq!(Score::of(i32::MAX).value(), MAX);
        assert_eq!(Score::initial().changed(i32::MAX).value(), MAX);
    }

    #[test]
    fn approval_earns_and_disapproval_loses() {
        assert_eq!(for_reaction(ReactionKind::Like), 1);
        assert_eq!(for_reaction(ReactionKind::Agree), 1);
        assert_eq!(for_reaction(ReactionKind::Thanks), 2);
        assert_eq!(for_reaction(ReactionKind::Disagree), -1);
    }

    #[test]
    fn every_reaction_kind_carries_a_weight() {
        for kind in ReactionKind::all() {
            assert_ne!(for_reaction(kind), 0);
        }
    }

    #[test]
    fn a_deletion_costs_more_than_any_single_reaction() {
        let worst = ReactionKind::all()
            .into_iter()
            .map(|kind| for_reaction(kind).abs())
            .max()
            .expect("a kind");
        assert!(for_deletion().abs() > worst);
        assert!(for_deletion() < 0);
    }

    #[test]
    fn scores_compare_by_value() {
        assert!(Score::of(5) > Score::of(4));
        assert_eq!(Score::of(7), Score::of(7));
    }
}
