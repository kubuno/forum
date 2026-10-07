-- V0 security hardening.

-- SEC-15: a member may hold at most one *pending* report per post, so the
-- moderation queue cannot be flooded by reporting the same message over and
-- over. A report that has been resolved or rejected no longer blocks a fresh
-- one, so a message that goes bad again can still be flagged.

-- Collapse any pre-existing duplicate pending reports (keep the earliest) so the
-- unique index below can be created even on a board that already accumulated
-- some before this rule existed.
DELETE FROM forum.reports a
      USING forum.reports b
      WHERE a.status = 'pending'
        AND b.status = 'pending'
        AND a.post_id = b.post_id
        AND a.reporter_id = b.reporter_id
        AND (a.created_at > b.created_at
             OR (a.created_at = b.created_at AND a.id > b.id));

CREATE UNIQUE INDEX IF NOT EXISTS uq_forum_reports_pending
    ON forum.reports (post_id, reporter_id)
    WHERE status = 'pending';
