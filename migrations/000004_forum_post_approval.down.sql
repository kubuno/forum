DROP INDEX IF EXISTS forum.idx_forum_topics_pending;
DROP INDEX IF EXISTS forum.idx_forum_posts_pending;

ALTER TABLE forum.topics
    DROP COLUMN IF EXISTS approved_by,
    DROP COLUMN IF EXISTS approved_at;

ALTER TABLE forum.posts
    DROP COLUMN IF EXISTS approved_by,
    DROP COLUMN IF EXISTS approved_at;
