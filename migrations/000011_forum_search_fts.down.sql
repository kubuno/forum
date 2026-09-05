DROP INDEX IF EXISTS forum.idx_forum_topic_search;
DROP INDEX IF EXISTS forum.idx_forum_post_search;

ALTER TABLE forum.topics DROP COLUMN IF EXISTS search_vector;
ALTER TABLE forum.posts  DROP COLUMN IF EXISTS search_vector;
