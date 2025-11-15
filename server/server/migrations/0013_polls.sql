CREATE TABLE polls (
    id UUID PRIMARY KEY,
    topic_id UUID NOT NULL UNIQUE REFERENCES topics (id),
    question TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE poll_options (
    id UUID PRIMARY KEY,
    poll_id UUID NOT NULL REFERENCES polls (id),
    text TEXT NOT NULL,
    position INT NOT NULL
);

CREATE INDEX poll_options_poll_idx ON poll_options (poll_id, position);

CREATE TABLE poll_votes (
    poll_id UUID NOT NULL REFERENCES polls (id),
    user_id UUID NOT NULL REFERENCES users (id),
    option_id UUID NOT NULL REFERENCES poll_options (id),
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (poll_id, user_id)
);

CREATE INDEX poll_votes_option_idx ON poll_votes (option_id);
