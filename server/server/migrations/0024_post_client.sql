ALTER TABLE post_events ADD COLUMN client TEXT;

CREATE INDEX post_events_recent_idx ON post_events (subject, created_at DESC);
