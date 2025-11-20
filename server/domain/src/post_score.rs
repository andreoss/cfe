use crate::Score;

pub const UNRESTRICTED: i32 = -9999;
pub const REGISTERED: i32 = -50;
pub const FLOOR_50: i32 = 50;
pub const FLOOR_100: i32 = 100;
pub const FLOOR_200: i32 = 200;
pub const FLOOR_300: i32 = 300;
pub const FLOOR_400: i32 = 400;
pub const FLOOR_500: i32 = 500;
pub const MODERATOR_OR_AUTHOR: i32 = 9999;
pub const MODERATORS_ONLY: i32 = 10_000;
pub const NO_COMMENTS: i32 = 10_001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostScore {
    Unrestricted,
    Registered,
    Floor(i32),
    ModeratorOrAuthor,
    ModeratorsOnly,
    NoComments,
}

impl PostScore {
    pub fn from_db(value: i32) -> Self {
        match value {
            UNRESTRICTED => PostScore::Unrestricted,
            REGISTERED => PostScore::Registered,
            MODERATOR_OR_AUTHOR => PostScore::ModeratorOrAuthor,
            MODERATORS_ONLY => PostScore::ModeratorsOnly,
            NO_COMMENTS => PostScore::NoComments,
            n if n >= FLOOR_50 => PostScore::Floor(n),
            _ => PostScore::Floor(0),
        }
    }

    pub fn to_db(&self) -> i32 {
        match self {
            PostScore::Unrestricted => UNRESTRICTED,
            PostScore::Registered => REGISTERED,
            PostScore::Floor(n) => *n,
            PostScore::ModeratorOrAuthor => MODERATOR_OR_AUTHOR,
            PostScore::ModeratorsOnly => MODERATORS_ONLY,
            PostScore::NoComments => NO_COMMENTS,
        }
    }

    pub fn restrictive_rank(&self) -> i32 {
        self.to_db()
    }

    pub fn more_restrictive(left: PostScore, right: PostScore) -> PostScore {
        if left.restrictive_rank() >= right.restrictive_rank() {
            left
        } else {
            right
        }
    }

    pub fn allows(&self, score: Score, is_moderator: bool, by_author: bool) -> bool {
        match self {
            PostScore::Unrestricted => true,
            PostScore::Registered => true,
            PostScore::Floor(threshold) => {
                is_moderator || by_author || score.value() >= *threshold
            }
            PostScore::ModeratorOrAuthor => is_moderator || by_author,
            PostScore::ModeratorsOnly => is_moderator,
            PostScore::NoComments => false,
        }
    }
}

impl Default for PostScore {
    fn default() -> Self {
        PostScore::Unrestricted
    }
}

pub fn thread_size_restriction(comment_count: u64) -> PostScore {
    if comment_count > 3000 {
        PostScore::Floor(FLOOR_200)
    } else if comment_count > 2000 {
        PostScore::Floor(FLOOR_100)
    } else if comment_count > 1000 {
        PostScore::Floor(FLOOR_50)
    } else {
        PostScore::Unrestricted
    }
}

pub fn comment_restriction(
    topic_score: PostScore,
    section_score: PostScore,
    comment_count: u64,
) -> PostScore {
    let thread = thread_size_restriction(comment_count);
    PostScore::more_restrictive(
        PostScore::more_restrictive(topic_score, section_score),
        thread,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinels_are_lossless() {
        for value in [
            UNRESTRICTED,
            REGISTERED,
            FLOOR_50,
            FLOOR_100,
            FLOOR_200,
            FLOOR_500,
            MODERATOR_OR_AUTHOR,
            MODERATORS_ONLY,
            NO_COMMENTS,
        ] {
            assert_eq!(PostScore::from_db(value).to_db(), value);
        }
    }

    #[test]
    fn unrestricted_allows_anyone() {
        let score = Score::initial();
        assert!(PostScore::Unrestricted.allows(score, false, false));
    }

    #[test]
    fn moderators_only_allows_only_moderators() {
        assert!(PostScore::ModeratorsOnly.allows(Score::initial(), true, false));
        assert!(!PostScore::ModeratorsOnly.allows(Score::initial(), false, false));
        assert!(!PostScore::ModeratorsOnly.allows(Score::initial(), false, true));
    }

    #[test]
    fn no_comments_allows_nobody() {
        assert!(!PostScore::NoComments.allows(Score::initial(), true, false));
        assert!(!PostScore::NoComments.allows(Score::initial(), false, true));
    }

    #[test]
    fn floor_requires_the_score() {
        assert!(PostScore::Floor(FLOOR_50).allows(Score::of(50), false, false));
        assert!(!PostScore::Floor(FLOOR_50).allows(Score::of(49), false, false));
    }

    #[test]
    fn moderator_and_author_bypass_any_floor() {
        let gate = PostScore::Floor(FLOOR_500);
        assert!(gate.allows(Score::initial(), true, false));
        assert!(gate.allows(Score::initial(), false, true));
    }

    #[test]
    fn moderator_or_author_lets_either_through() {
        assert!(PostScore::ModeratorOrAuthor.allows(Score::initial(), true, false));
        assert!(PostScore::ModeratorOrAuthor.allows(Score::initial(), false, true));
        assert!(!PostScore::ModeratorOrAuthor.allows(Score::initial(), false, false));
    }

    #[test]
    fn most_restrictive_wins() {
        assert_eq!(
            PostScore::more_restrictive(PostScore::Unrestricted, PostScore::Floor(FLOOR_50)),
            PostScore::Floor(FLOOR_50)
        );
        assert_eq!(
            PostScore::more_restrictive(PostScore::NoComments, PostScore::Floor(FLOOR_50)),
            PostScore::NoComments
        );
        assert_eq!(
            PostScore::more_restrictive(PostScore::Floor(FLOOR_100), PostScore::Floor(FLOOR_50)),
            PostScore::Floor(FLOOR_100)
        );
    }

    #[test]
    fn thread_size_tightens_in_steps() {
        assert_eq!(
            thread_size_restriction(500),
            PostScore::Unrestricted
        );
        assert_eq!(
            thread_size_restriction(1001),
            PostScore::Floor(FLOOR_50)
        );
        assert_eq!(
            thread_size_restriction(2001),
            PostScore::Floor(FLOOR_100)
        );
        assert_eq!(
            thread_size_restriction(3001),
            PostScore::Floor(FLOOR_200)
        );
    }

    #[test]
    fn comment_restriction_takes_the_strictest_input() {
        let strict = comment_restriction(
            PostScore::Unrestricted,
            PostScore::Unrestricted,
            500,
        );
        assert_eq!(strict, PostScore::Unrestricted);
        let by_thread = comment_restriction(
            PostScore::Unrestricted,
            PostScore::Unrestricted,
            1001,
        );
        assert_eq!(by_thread, PostScore::Floor(FLOOR_50));
        let by_topic = comment_restriction(
            PostScore::ModeratorsOnly,
            PostScore::Unrestricted,
            500,
        );
        assert_eq!(by_topic, PostScore::ModeratorsOnly);
        let by_section = comment_restriction(
            PostScore::Unrestricted,
            PostScore::Floor(FLOOR_100),
            500,
        );
        assert_eq!(by_section, PostScore::Floor(FLOOR_100));
    }
}