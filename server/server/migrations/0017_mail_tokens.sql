ALTER TABLE users ADD COLUMN confirmed_at TIMESTAMPTZ;

CREATE TABLE mail_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id),
    purpose TEXT NOT NULL,
    digest TEXT NOT NULL UNIQUE,
    payload TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    redeemed_at TIMESTAMPTZ
);

CREATE INDEX mail_tokens_user_idx ON mail_tokens (user_id, purpose);
