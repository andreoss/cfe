CREATE TABLE invitations (
    id UUID PRIMARY KEY,
    code VARCHAR(32) NOT NULL UNIQUE,
    issuer_id UUID NOT NULL REFERENCES users (id),
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    spent_at TIMESTAMPTZ,
    spent_by UUID REFERENCES users (id)
);

CREATE INDEX invitations_issuer_idx ON invitations (issuer_id, created_at DESC);
