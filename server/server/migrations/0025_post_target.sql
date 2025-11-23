ALTER TABLE post_events ADD COLUMN topic_id UUID;

ALTER TABLE post_events ADD COLUMN comment_id UUID;

CREATE INDEX post_events_target_idx ON post_events (subject, created_at DESC, topic_id, comment_id);
