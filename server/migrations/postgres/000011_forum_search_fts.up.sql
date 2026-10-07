-- Replaces the ILIKE substring search with real Postgres full-text search.
-- Generated, stored tsvector columns keep the index in sync with the source
-- text automatically (no trigger to maintain) and let the planner use a GIN
-- index instead of a sequential scan with a leading wildcard.
--
-- NOTE: the text-configuration argument is cast to `regconfig`. The two-arg
-- `to_tsvector('french', …)` form looks the configuration up by name at run
-- time and is only STABLE, which Postgres rejects in a generated column
-- ("generation expression is not immutable"); `'french'::regconfig` folds to the
-- configuration's OID at DDL time, making the whole expression IMMUTABLE.

ALTER TABLE forum.posts
    ADD COLUMN search_vector tsvector
        GENERATED ALWAYS AS (to_tsvector('french'::regconfig, coalesce(body_md, ''))) STORED;

ALTER TABLE forum.topics
    ADD COLUMN search_vector tsvector
        GENERATED ALWAYS AS (to_tsvector('french'::regconfig, coalesce(title, ''))) STORED;

CREATE INDEX idx_forum_post_search  ON forum.posts  USING GIN (search_vector);
CREATE INDEX idx_forum_topic_search ON forum.topics USING GIN (search_vector);
