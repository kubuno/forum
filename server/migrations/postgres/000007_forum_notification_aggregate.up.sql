-- Notification aggregation: a busy topic no longer buries a member under one
-- unread notification per reply. High-volume, topic-scoped kinds (a reply, a
-- reaction) fold into the recipient's existing unread notification for that
-- topic; `responder_count` records how many people it now stands for, so the
-- bell can read "X and 4 others replied".
ALTER TABLE forum.notifications
    ADD COLUMN IF NOT EXISTS responder_count INT NOT NULL DEFAULT 1;

-- Speeds up the fold-in lookup (recipient's open notification for a topic+kind).
CREATE INDEX IF NOT EXISTS idx_forum_notif_fold
    ON forum.notifications(user_id, kind, topic_id) WHERE is_read = FALSE;
