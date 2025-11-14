CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    recipient_id UUID NOT NULL REFERENCES users (id),
    actor_id UUID NOT NULL REFERENCES users (id),
    topic_id UUID NOT NULL REFERENCES topics (id),
    comment_id UUID NOT NULL REFERENCES comments (id),
    created_at TIMESTAMPTZ NOT NULL,
    read_at TIMESTAMPTZ
);

CREATE INDEX notifications_recipient_idx ON notifications (recipient_id, created_at DESC);
