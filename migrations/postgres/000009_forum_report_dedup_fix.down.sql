DROP INDEX IF EXISTS forum.uq_forum_reports_open;
CREATE UNIQUE INDEX IF NOT EXISTS uq_forum_reports_pending
    ON forum.reports (post_id, reporter_id)
    WHERE status = 'pending';
