-- SEC-12: topics gain a soft-delete, matching posts. Deleting a topic (by its
-- author or a moderator) no longer erases it and cascades away its posts; it is
-- flagged instead, drops out of every listing, and can be recovered later. A
-- permanent purge is a separate, moderator-only action added with the trash view.
ALTER TABLE forum.topics
    ADD COLUMN IF NOT EXISTS is_deleted    BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deleted_at    TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deleted_by    UUID,
    ADD COLUMN IF NOT EXISTS delete_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_forum_topic_deleted
    ON forum.topics(forum_id) WHERE is_deleted = TRUE;
