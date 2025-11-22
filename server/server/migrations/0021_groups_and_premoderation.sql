CREATE TABLE groups (
    id UUID PRIMARY KEY,
    section_id UUID NOT NULL REFERENCES sections (id),
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    UNIQUE (section_id, slug)
);

ALTER TABLE topics ADD COLUMN group_id UUID REFERENCES groups (id);
ALTER TABLE topics ADD COLUMN pending BOOLEAN NOT NULL DEFAULT FALSE;
