CREATE TABLE sections (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL
);

CREATE TABLE topics (
    id UUID PRIMARY KEY,
    section_id UUID NOT NULL REFERENCES sections (id),
    author_id UUID NOT NULL REFERENCES users (id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO sections (id, slug, title) VALUES
    ('00000000-0000-0000-0000-000000000001', 'general', 'General'),
    ('00000000-0000-0000-0000-000000000002', 'help', 'Help'),
    ('00000000-0000-0000-0000-000000000003', 'feedback', 'Feedback');
