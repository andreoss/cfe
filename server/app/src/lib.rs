mod account;
mod activity;
mod avatars;
mod bookmarks;
mod comments;
mod editing;
mod enforcement;
mod lifecycle;
mod moderation;
mod notifications;
mod polls;
mod paging;
mod ports;
mod profile;
mod reactions;
mod register;
mod search;
mod session;
mod sign_in;
mod test_support;
mod topics;

pub use activity::recent_activity;
pub use account::{ChangePasswordError, change_password, deregister};
pub use avatars::{AvatarLookupError, clear_avatar, get_avatar, has_avatar, set_avatar};
pub use bookmarks::{
    BookmarkError, add_bookmark, is_bookmarked, list_bookmarked_topics, remove_bookmark,
};
pub use comments::{PostCommentError, list_comments, post_comment};
pub use editing::{EditError, edit_comment, edit_topic};
pub use enforcement::{
    EnforcementError, acknowledge_warnings, active_ban, ban_user, ignore_user, ignored_by,
    lift_ban, list_warnings, promote_to_moderator, stop_ignoring, warn_user,
};
pub use lifecycle::{
    ChangeEmailError, RedeemError, confirm_activation, confirm_email_change, request_activation,
    request_email_change, request_password_reset, reset_password,
};
pub use moderation::{DeleteError, delete_comment, delete_topic};
pub use notifications::{MarkReadError, count_unread, list_notifications, mark_read};
pub use polls::{
    CreatePollError, PollResults, VoteError, cast_vote, create_poll, poll_results,
};
pub use paging::Paged;
pub use ports::{
    ActivityRepository, AvatarRepository, BookmarkRepository, CommentRepository,
    EnforcementRepository,
    NotificationRepository, PasswordHasher, PollRepository,
    ReactionRepository, SearchRepository, SectionRepository, SessionRepository, TopicRepository,
    UserRepository,
};
pub use profile::{UpdateBioError, update_bio};
pub use reactions::{ReactionSummary, clear_reaction, react, summarize_reactions};
pub use register::{RegisterError, register};
pub use search::search;
pub use session::{create_session, current_user, sign_out, touch_session};
pub use sign_in::{SignInError, sign_in};
pub use topics::{
    CreateTopicError, ListTopicsError, create_topic, get_topic, list_sections, list_topics,
    list_topics_by_tag,
};
