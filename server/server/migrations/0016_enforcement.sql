CREATE TABLE bans (
    user_id UUID PRIMARY KEY REFERENCES users (id),
    moderator_id UUID NOT NULL REFERENCES users (id),
    reason TEXT NOT NULL,
    banned_at TIMESTAMPTZ NOT NULL,
    until TIMESTAMPTZ
);

CREATE TABLE warnings (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id),
    moderator_id UUID NOT NULL REFERENCES users (id),
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX warnings_user_idx ON warnings (user_id, created_at DESC);

CREATE TABLE ignores (
    user_id UUID NOT NULL REFERENCES users (id),
    ignored_id UUID NOT NULL REFERENCES users (id),
    PRIMARY KEY (user_id, ignored_id)
);
