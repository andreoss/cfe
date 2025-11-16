CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    email VARCHAR NOT NULL UNIQUE,
    password_hash VARCHAR NOT NULL,
    bio VARCHAR,
    role VARCHAR NOT NULL DEFAULT 'user',
    deregistered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    token VARCHAR NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE sections (
    id UUID PRIMARY KEY,
    slug VARCHAR NOT NULL UNIQUE,
    title VARCHAR NOT NULL
);

CREATE TABLE topics (
    id UUID PRIMARY KEY,
    section_id UUID NOT NULL,
    author_id UUID NOT NULL,
    title VARCHAR NOT NULL,
    body VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    deleted_reason VARCHAR,
    deleted_by UUID,
    deleted_at TIMESTAMPTZ,
    edited_by UUID,
    edited_at TIMESTAMPTZ
);

CREATE TABLE topic_tags (
    topic_id UUID NOT NULL,
    tag VARCHAR NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (topic_id, tag)
);

CREATE INDEX topic_tags_tag_idx ON topic_tags (tag);

CREATE TABLE comments (
    id UUID PRIMARY KEY,
    topic_id UUID NOT NULL,
    author_id UUID NOT NULL,
    parent_id UUID,
    body VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    deleted_reason VARCHAR,
    deleted_by UUID,
    deleted_at TIMESTAMPTZ,
    edited_by UUID,
    edited_at TIMESTAMPTZ
);

CREATE INDEX comments_topic_idx ON comments (topic_id, created_at);

CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    recipient_id UUID NOT NULL,
    actor_id UUID NOT NULL,
    topic_id UUID NOT NULL,
    comment_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    read_at TIMESTAMPTZ
);

CREATE INDEX notifications_recipient_idx ON notifications (recipient_id, created_at);

CREATE TABLE bookmarks (
    user_id UUID NOT NULL,
    topic_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, topic_id)
);

CREATE TABLE reactions (
    user_id UUID NOT NULL,
    target_kind VARCHAR NOT NULL,
    target_id UUID NOT NULL,
    kind VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, target_kind, target_id)
);

CREATE INDEX reactions_target_idx ON reactions (target_kind, target_id);

CREATE TABLE polls (
    id UUID PRIMARY KEY,
    topic_id UUID NOT NULL UNIQUE,
    question VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE poll_options (
    id UUID PRIMARY KEY,
    poll_id UUID NOT NULL,
    text VARCHAR NOT NULL,
    position INTEGER NOT NULL
);

CREATE INDEX poll_options_poll_idx ON poll_options (poll_id, position);

CREATE TABLE poll_votes (
    poll_id UUID NOT NULL,
    user_id UUID NOT NULL,
    option_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (poll_id, user_id)
);

CREATE INDEX poll_votes_option_idx ON poll_votes (option_id);

CREATE TABLE avatars (
    user_id UUID PRIMARY KEY,
    bytes BLOB NOT NULL,
    content_type VARCHAR NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE bans (
    user_id UUID PRIMARY KEY,
    moderator_id UUID NOT NULL,
    reason VARCHAR NOT NULL,
    banned_at TIMESTAMPTZ NOT NULL,
    until TIMESTAMPTZ
);

CREATE TABLE warnings (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    moderator_id UUID NOT NULL,
    reason VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX warnings_user_idx ON warnings (user_id, created_at);

CREATE TABLE ignores (
    user_id UUID NOT NULL,
    ignored_id UUID NOT NULL,
    PRIMARY KEY (user_id, ignored_id)
);

INSERT INTO sections (id, slug, title) VALUES
    ('00000000-0000-0000-0000-000000000001', 'general', 'General'),
    ('00000000-0000-0000-0000-000000000002', 'help', 'Help'),
    ('00000000-0000-0000-0000-000000000003', 'feedback', 'Feedback');
