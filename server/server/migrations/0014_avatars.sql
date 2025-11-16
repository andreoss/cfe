CREATE TABLE avatars (
    user_id UUID PRIMARY KEY REFERENCES users (id),
    bytes BYTEA NOT NULL,
    content_type TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
