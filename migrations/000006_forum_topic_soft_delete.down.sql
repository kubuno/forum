DROP INDEX IF EXISTS forum.idx_forum_topic_deleted;
ALTER TABLE forum.topics
    DROP COLUMN IF EXISTS is_deleted,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS deleted_by,
    DROP COLUMN IF EXISTS delete_reason;
