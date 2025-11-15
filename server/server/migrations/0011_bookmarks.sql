CREATE TABLE bookmarks (
    user_id UUID NOT NULL REFERENCES users (id),
    topic_id UUID NOT NULL REFERENCES topics (id),
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, topic_id)
);

CREATE INDEX bookmarks_user_idx ON bookmarks (user_id, created_at DESC);
