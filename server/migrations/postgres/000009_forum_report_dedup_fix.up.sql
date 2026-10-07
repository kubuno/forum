-- SEC-15 fix. The one-pending-report-per-(post, reporter) guard added in
-- migration 000005 keyed on `status = 'pending'`, but `forum.reports.status`
-- (migration 000001) only ever allows 'open' | 'resolved' | 'rejected', with
-- 'open' as the default open state. The old partial index therefore matched no
-- row and the guard was dead. Recreate it on the real open state.
DROP INDEX IF EXISTS forum.uq_forum_reports_pending;

-- Collapse any pre-existing duplicate OPEN reports (keep the earliest) so the
-- new unique index can be created on a board that already accumulated some.
DELETE FROM forum.reports a
      USING forum.reports b
      WHERE a.status = 'open'
        AND b.status = 'open'
        AND a.post_id = b.post_id
        AND a.reporter_id = b.reporter_id
        AND (a.created_at > b.created_at
             OR (a.created_at = b.created_at AND a.id > b.id));

CREATE UNIQUE INDEX IF NOT EXISTS uq_forum_reports_open
    ON forum.reports (post_id, reporter_id)
    WHERE status = 'open';
