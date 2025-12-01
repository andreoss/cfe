CREATE TABLE versions (
    id UUID PRIMARY KEY,
    subject_kind VARCHAR(16) NOT NULL,
    subject_id UUID NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    editor_id UUID NOT NULL REFERENCES users (id),
    written_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX versions_subject_idx ON versions (subject_kind, subject_id, written_at);
