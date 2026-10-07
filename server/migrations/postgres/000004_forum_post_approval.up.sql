-- Approval queue for contributions.
--
-- `is_approved` already existed on `forum.topics` and `forum.posts` (000001) but
-- nothing ever wrote it or read it: every row was born approved and stayed so.
-- The instance setting `forum.post_approval_mode` now holds contributions back,
-- so the column becomes load-bearing and needs the two things a queue needs:
--   * a way to list what is waiting, without scanning every message ever posted;
--   * a record of who released it and when, so the moderation log is not the
--     only trace of a decision that changed what the forum shows.

ALTER TABLE forum.posts
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS approved_by UUID;

ALTER TABLE forum.topics
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS approved_by UUID;

-- Partial indexes: the queue is the exception, never the bulk of the table, so
-- only the pending rows are indexed. On an unmoderated instance these stay
-- empty and cost nothing.
CREATE INDEX IF NOT EXISTS idx_forum_posts_pending
    ON forum.posts(created_at)
    WHERE is_approved = FALSE;

CREATE INDEX IF NOT EXISTS idx_forum_topics_pending
    ON forum.topics(created_at)
    WHERE is_approved = FALSE;

-- Rows that predate the queue were all visible; stamp them so "approved with no
-- approver" reads as "published before moderation existed" rather than as a
-- decision nobody signed.
UPDATE forum.posts  SET approved_at = created_at WHERE is_approved = TRUE AND approved_at IS NULL;
UPDATE forum.topics SET approved_at = created_at WHERE is_approved = TRUE AND approved_at IS NULL;
