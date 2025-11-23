CREATE TABLE sign_in_failures (
    username TEXT NOT NULL,
    addr TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sign_in_failures_subject_idx ON sign_in_failures (username, addr, created_at);

CREATE TABLE known_addresses (
    user_id UUID NOT NULL REFERENCES users (id),
    addr TEXT NOT NULL,
    first_seen TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, addr)
);
