CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    bio TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    deregistered_at BIGINT,
    confirmed_at BIGINT,
    score INTEGER NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER) * 1000000)
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at BIGINT NOT NULL
);

CREATE TABLE sections (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    topics_score INTEGER NOT NULL DEFAULT -9999
);
CREATE TABLE groups (
    id TEXT PRIMARY KEY,
    section_id TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    UNIQUE (section_id, slug)
);
CREATE TABLE topics (
    id TEXT PRIMARY KEY,
    section_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    postscore INTEGER NOT NULL DEFAULT -9999,
    deleted_reason TEXT,
    deleted_by TEXT,
    deleted_at BIGINT,
    deletion_penalty INTEGER NOT NULL DEFAULT -10,
    edited_by TEXT,
    edited_at BIGINT,
    group_id TEXT,
    pending BOOLEAN NOT NULL DEFAULT FALSE,
    draft BOOLEAN NOT NULL DEFAULT FALSE,
    sticky BOOLEAN NOT NULL DEFAULT FALSE,
    off_front BOOLEAN NOT NULL DEFAULT FALSE,
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    minor BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX topics_section_sticky_idx ON topics (section_id, sticky, created_at);

CREATE TABLE topic_tags (
    topic_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (topic_id, tag)
);

CREATE INDEX topic_tags_tag_idx ON topic_tags (tag);

CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    parent_id TEXT,
    body TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    deleted_reason TEXT,
    deleted_by TEXT,
    deleted_at BIGINT,
    deletion_penalty INTEGER NOT NULL DEFAULT -10,
    edited_by TEXT,
    edited_at BIGINT
);

CREATE INDEX comments_topic_idx ON comments (topic_id, created_at);

CREATE TABLE notifications (
    id TEXT PRIMARY KEY,
    recipient_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    comment_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    read_at BIGINT,
    kind TEXT NOT NULL DEFAULT 'reply'
);

CREATE INDEX notifications_recipient_idx ON notifications (recipient_id, created_at);

CREATE TABLE bookmarks (
    user_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (user_id, topic_id)
);

CREATE TABLE watches (
    user_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (user_id, topic_id)
);

CREATE INDEX watches_user_idx ON watches (user_id, created_at DESC);

CREATE INDEX watches_topic_idx ON watches (topic_id);

CREATE TABLE remarks (
    author_id TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (author_id, subject_id)
);

CREATE INDEX remarks_author_idx ON remarks (author_id, created_at DESC);

CREATE TABLE invitations (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    issuer_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    spent_at BIGINT,
    spent_by TEXT
);

CREATE INDEX invitations_issuer_idx ON invitations (issuer_id, created_at DESC);

CREATE TABLE versions (
    id TEXT PRIMARY KEY,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    editor_id TEXT NOT NULL,
    written_at BIGINT NOT NULL
);

CREATE INDEX versions_subject_idx ON versions (subject_kind, subject_id, written_at);

CREATE TABLE tags (
    slug TEXT PRIMARY KEY,
    description TEXT,
    means TEXT
);

CREATE TABLE tag_follows (
    user_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    PRIMARY KEY (user_id, slug)
);

CREATE INDEX tag_follows_slug_idx ON tag_follows (slug);

CREATE TABLE attachments (
    id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    content_type TEXT NOT NULL,
    bytes BLOB NOT NULL,
    uploaded_by TEXT NOT NULL,
    uploaded_at BIGINT NOT NULL
);

CREATE INDEX attachments_topic_idx ON attachments (topic_id, uploaded_at);

CREATE TABLE reactions (
    user_id TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (user_id, target_kind, target_id)
);

CREATE INDEX reactions_target_idx ON reactions (target_kind, target_id);

CREATE TABLE polls (
    id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL UNIQUE,
    question TEXT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE TABLE poll_options (
    id TEXT PRIMARY KEY,
    poll_id TEXT NOT NULL,
    text TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE INDEX poll_options_poll_idx ON poll_options (poll_id, position);

CREATE TABLE poll_votes (
    poll_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    option_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (poll_id, user_id)
);

CREATE INDEX poll_votes_option_idx ON poll_votes (option_id);

CREATE TABLE avatars (
    user_id TEXT PRIMARY KEY,
    bytes BLOB NOT NULL,
    content_type TEXT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE bans (
    user_id TEXT PRIMARY KEY,
    moderator_id TEXT NULL,
    reason TEXT NOT NULL,
    banned_at BIGINT NOT NULL,
    until BIGINT
);

CREATE TABLE warnings (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    moderator_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX warnings_user_idx ON warnings (user_id, created_at);

CREATE TABLE ignores (
    user_id TEXT NOT NULL,
    ignored_id TEXT NOT NULL,
    PRIMARY KEY (user_id, ignored_id)
);

INSERT INTO sections (id, slug, title) VALUES
    ('00000000-0000-0000-0000-000000000001', 'general', 'General'),
    ('00000000-0000-0000-0000-000000000002', 'help', 'Help'),
    ('00000000-0000-0000-0000-000000000003', 'feedback', 'Feedback');

CREATE TABLE mail_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    purpose TEXT NOT NULL,
    digest TEXT NOT NULL UNIQUE,
    payload TEXT,
    expires_at BIGINT NOT NULL,
    redeemed_at BIGINT
);

CREATE INDEX mail_tokens_user_idx ON mail_tokens (user_id, purpose);

CREATE TABLE address_blocks (
    addr TEXT PRIMARY KEY,
    moderator_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    blocked_at BIGINT NOT NULL,
    until BIGINT,
    mode TEXT NOT NULL DEFAULT 'refuse'
);

CREATE TABLE post_events (
    subject TEXT NOT NULL,
    kind TEXT NOT NULL,
    user_id TEXT,
    client TEXT,
    created_at BIGINT NOT NULL,
    topic_id TEXT,
    comment_id TEXT
);

CREATE INDEX post_events_subject_idx ON post_events (subject, created_at);

CREATE INDEX post_events_recent_idx ON post_events (subject, created_at DESC);

CREATE INDEX post_events_target_idx ON post_events (subject, created_at DESC, topic_id, comment_id);

CREATE TABLE reports (
    id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    comment_id TEXT,
    reporter_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    closed_by TEXT,
    closed_at BIGINT
);

CREATE INDEX reports_open_idx ON reports (closed_at, created_at);

CREATE INDEX reports_topic_idx ON reports (topic_id, closed_at);

CREATE TABLE sign_in_failures (
    username TEXT NOT NULL,
    addr TEXT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE INDEX sign_in_failures_subject_idx ON sign_in_failures (username, addr, created_at);

CREATE TABLE known_addresses (
    user_id TEXT NOT NULL,
    addr TEXT NOT NULL,
    first_seen BIGINT NOT NULL,
    PRIMARY KEY (user_id, addr)
);
