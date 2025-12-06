CREATE TABLE attachments (
    id UUID PRIMARY KEY,
    topic_id UUID NOT NULL REFERENCES topics (id),
    content_type VARCHAR(32) NOT NULL,
    bytes BYTEA NOT NULL,
    uploaded_by UUID NOT NULL REFERENCES users (id),
    uploaded_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX attachments_topic_idx ON attachments (topic_id, uploaded_at);
