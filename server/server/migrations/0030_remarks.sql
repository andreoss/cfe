CREATE TABLE remarks (
    author_id UUID NOT NULL REFERENCES users (id),
    subject_id UUID NOT NULL REFERENCES users (id),
    text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (author_id, subject_id)
);

CREATE INDEX remarks_author_idx ON remarks (author_id, created_at DESC);
