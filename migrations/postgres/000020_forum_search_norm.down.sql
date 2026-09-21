DROP INDEX IF EXISTS forum.idx_forum_topic_title_norm;
DROP INDEX IF EXISTS forum.idx_forum_post_body_norm;
ALTER TABLE forum.topics DROP COLUMN IF EXISTS title_norm;
ALTER TABLE forum.posts  DROP COLUMN IF EXISTS body_norm;

ALTER TABLE forum.posts
    ADD COLUMN search_vector tsvector
        GENERATED ALWAYS AS (to_tsvector('french'::regconfig, coalesce(body_md, ''))) STORED;
ALTER TABLE forum.topics
    ADD COLUMN search_vector tsvector
        GENERATED ALWAYS AS (to_tsvector('french'::regconfig, coalesce(title, ''))) STORED;
CREATE INDEX idx_forum_post_search  ON forum.posts  USING GIN (search_vector);
CREATE INDEX idx_forum_topic_search ON forum.topics USING GIN (search_vector);
