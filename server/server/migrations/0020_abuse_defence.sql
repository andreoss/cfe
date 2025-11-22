CREATE TABLE address_blocks (
    addr TEXT PRIMARY KEY,
    moderator_id UUID NOT NULL REFERENCES users (id),
    reason TEXT NOT NULL,
    blocked_at TIMESTAMPTZ NOT NULL,
    until TIMESTAMPTZ
);

CREATE TABLE post_events (
    subject TEXT NOT NULL,
    kind TEXT NOT NULL,
    user_id UUID,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX post_events_subject_idx ON post_events (subject, created_at);