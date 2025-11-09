ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

ALTER TABLE topics ADD COLUMN deleted_reason TEXT;
ALTER TABLE topics ADD COLUMN deleted_by UUID REFERENCES users (id);
ALTER TABLE topics ADD COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE comments ADD COLUMN deleted_reason TEXT;
ALTER TABLE comments ADD COLUMN deleted_by UUID REFERENCES users (id);
ALTER TABLE comments ADD COLUMN deleted_at TIMESTAMPTZ;
