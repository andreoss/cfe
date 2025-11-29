CREATE TABLE watches (
    user_id UUID NOT NULL REFERENCES users (id),
    topic_id UUID NOT NULL REFERENCES topics (id),
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, topic_id)
);

CREATE INDEX watches_user_idx ON watches (user_id, created_at DESC);

CREATE INDEX watches_topic_idx ON watches (topic_id);

ALTER TABLE notifications ADD COLUMN kind TEXT NOT NULL DEFAULT 'reply';
