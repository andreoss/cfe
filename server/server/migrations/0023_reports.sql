CREATE TABLE reports (
    id UUID PRIMARY KEY,
    topic_id UUID NOT NULL REFERENCES topics (id),
    comment_id UUID REFERENCES comments (id),
    reporter_id UUID NOT NULL REFERENCES users (id),
    kind TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    closed_by UUID REFERENCES users (id),
    closed_at TIMESTAMPTZ
);

CREATE INDEX reports_open_idx ON reports (closed_at, created_at);

CREATE INDEX reports_topic_idx ON reports (topic_id, closed_at);
