DROP INDEX IF EXISTS forum.idx_forum_notif_fold;
ALTER TABLE forum.notifications DROP COLUMN IF EXISTS responder_count;
