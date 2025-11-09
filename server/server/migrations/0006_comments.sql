CREATE TABLE comments (
    id UUID PRIMARY KEY,
    topic_id UUID NOT NULL REFERENCES topics (id),
    author_id UUID NOT NULL REFERENCES users (id),
    parent_id UUID REFERENCES comments (id),
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
