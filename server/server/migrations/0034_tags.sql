CREATE TABLE tags (
    slug VARCHAR(64) PRIMARY KEY,
    description TEXT,
    means VARCHAR(64)
);

CREATE TABLE tag_follows (
    user_id UUID NOT NULL REFERENCES users (id),
    slug VARCHAR(64) NOT NULL,
    PRIMARY KEY (user_id, slug)
);

CREATE INDEX tag_follows_slug_idx ON tag_follows (slug);
