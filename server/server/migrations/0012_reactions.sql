CREATE TABLE reactions (
    user_id UUID NOT NULL REFERENCES users (id),
    target_kind TEXT NOT NULL,
    target_id UUID NOT NULL,
    kind TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, target_kind, target_id)
);

CREATE INDEX reactions_target_idx ON reactions (target_kind, target_id);
