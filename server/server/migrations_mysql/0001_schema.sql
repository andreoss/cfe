CREATE TABLE users (
    id BINARY(16) PRIMARY KEY,
    username VARCHAR(64) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    bio TEXT,
    role VARCHAR(16) NOT NULL DEFAULT 'user',
    deregistered_at TIMESTAMP(6) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE TABLE sessions (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    token VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMP(6) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE sections (
    id BINARY(16) PRIMARY KEY,
    slug VARCHAR(64) NOT NULL UNIQUE,
    title VARCHAR(255) NOT NULL
);

CREATE TABLE topics (
    id BINARY(16) PRIMARY KEY,
    section_id BINARY(16) NOT NULL,
    author_id BINARY(16) NOT NULL,
    title VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    deleted_reason TEXT,
    deleted_by BINARY(16),
    deleted_at TIMESTAMP(6) NULL,
    edited_by BINARY(16),
    edited_at TIMESTAMP(6) NULL,
    FOREIGN KEY (section_id) REFERENCES sections (id),
    FOREIGN KEY (author_id) REFERENCES users (id),
    FULLTEXT KEY topics_search_idx (title, body)
);

CREATE TABLE topic_tags (
    topic_id BINARY(16) NOT NULL,
    tag VARCHAR(64) NOT NULL,
    position INT NOT NULL,
    PRIMARY KEY (topic_id, tag),
    FOREIGN KEY (topic_id) REFERENCES topics (id)
);

CREATE INDEX topic_tags_tag_idx ON topic_tags (tag);

CREATE TABLE comments (
    id BINARY(16) PRIMARY KEY,
    topic_id BINARY(16) NOT NULL,
    author_id BINARY(16) NOT NULL,
    parent_id BINARY(16),
    body TEXT NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    deleted_reason TEXT,
    deleted_by BINARY(16),
    deleted_at TIMESTAMP(6) NULL,
    edited_by BINARY(16),
    edited_at TIMESTAMP(6) NULL,
    FOREIGN KEY (topic_id) REFERENCES topics (id),
    FOREIGN KEY (author_id) REFERENCES users (id),
    FULLTEXT KEY comments_search_idx (body)
);

CREATE INDEX comments_topic_idx ON comments (topic_id, created_at);

CREATE TABLE notifications (
    id BINARY(16) PRIMARY KEY,
    recipient_id BINARY(16) NOT NULL,
    actor_id BINARY(16) NOT NULL,
    topic_id BINARY(16) NOT NULL,
    comment_id BINARY(16) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    read_at TIMESTAMP(6) NULL,
    FOREIGN KEY (recipient_id) REFERENCES users (id),
    FOREIGN KEY (actor_id) REFERENCES users (id)
);

CREATE INDEX notifications_recipient_idx ON notifications (recipient_id, created_at);

CREATE TABLE bookmarks (
    user_id BINARY(16) NOT NULL,
    topic_id BINARY(16) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    PRIMARY KEY (user_id, topic_id),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (topic_id) REFERENCES topics (id)
);

CREATE TABLE reactions (
    user_id BINARY(16) NOT NULL,
    target_kind VARCHAR(16) NOT NULL,
    target_id BINARY(16) NOT NULL,
    kind VARCHAR(16) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    PRIMARY KEY (user_id, target_kind, target_id),
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE INDEX reactions_target_idx ON reactions (target_kind, target_id);

CREATE TABLE polls (
    id BINARY(16) PRIMARY KEY,
    topic_id BINARY(16) NOT NULL UNIQUE,
    question VARCHAR(255) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    FOREIGN KEY (topic_id) REFERENCES topics (id)
);

CREATE TABLE poll_options (
    id BINARY(16) PRIMARY KEY,
    poll_id BINARY(16) NOT NULL,
    text VARCHAR(255) NOT NULL,
    position INT NOT NULL,
    FOREIGN KEY (poll_id) REFERENCES polls (id)
);

CREATE INDEX poll_options_poll_idx ON poll_options (poll_id, position);

CREATE TABLE poll_votes (
    poll_id BINARY(16) NOT NULL,
    user_id BINARY(16) NOT NULL,
    option_id BINARY(16) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    PRIMARY KEY (poll_id, user_id),
    FOREIGN KEY (poll_id) REFERENCES polls (id),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (option_id) REFERENCES poll_options (id)
);

CREATE INDEX poll_votes_option_idx ON poll_votes (option_id);

CREATE TABLE avatars (
    user_id BINARY(16) PRIMARY KEY,
    bytes LONGBLOB NOT NULL,
    content_type VARCHAR(32) NOT NULL,
    updated_at TIMESTAMP(6) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE bans (
    user_id BINARY(16) PRIMARY KEY,
    moderator_id BINARY(16) NOT NULL,
    reason TEXT NOT NULL,
    banned_at TIMESTAMP(6) NOT NULL,
    until TIMESTAMP(6) NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (moderator_id) REFERENCES users (id)
);

CREATE TABLE warnings (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    moderator_id BINARY(16) NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMP(6) NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (moderator_id) REFERENCES users (id)
);

CREATE INDEX warnings_user_idx ON warnings (user_id, created_at);

CREATE TABLE ignores (
    user_id BINARY(16) NOT NULL,
    ignored_id BINARY(16) NOT NULL,
    PRIMARY KEY (user_id, ignored_id),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (ignored_id) REFERENCES users (id)
);

INSERT INTO sections (id, slug, title) VALUES
    (UNHEX(REPLACE('00000000-0000-0000-0000-000000000001', '-', '')), 'general', 'General'),
    (UNHEX(REPLACE('00000000-0000-0000-0000-000000000002', '-', '')), 'help', 'Help'),
    (UNHEX(REPLACE('00000000-0000-0000-0000-000000000003', '-', '')), 'feedback', 'Feedback');
