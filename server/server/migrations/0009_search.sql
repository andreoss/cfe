CREATE INDEX topics_search_idx ON topics
  USING GIN (to_tsvector('english', title || ' ' || body));

CREATE INDEX comments_search_idx ON comments
  USING GIN (to_tsvector('english', body));
