-- Make the conflict targets the services upsert against expressible on all three
-- engines. PostgreSQL's partial unique indexes (`... WHERE topic_id IS NOT
-- NULL`) have no MySQL equivalent, and `ON CONFLICT` can only name a
-- non-partial constraint through kubuno-db's portable helper.
--
-- Each of these targets is a pair where exactly one of two columns is ever set
-- (a subscription/draft watches a topic OR a forum, never both). A plain UNIQUE
-- over the pair behaves identically, because all three engines treat NULLs as
-- distinct in a unique index — so the rows carrying a NULL in the pair never
-- collide with one another.

-- ── Subscriptions ────────────────────────────────────────────────────────────
DROP INDEX IF EXISTS forum.idx_forum_sub_topic;
DROP INDEX IF EXISTS forum.idx_forum_sub_forum;
ALTER TABLE forum.subscriptions ADD CONSTRAINT uq_forum_sub_topic UNIQUE (user_id, topic_id);
ALTER TABLE forum.subscriptions ADD CONSTRAINT uq_forum_sub_forum UNIQUE (user_id, forum_id);

-- ── Drafts ───────────────────────────────────────────────────────────────────
DROP INDEX IF EXISTS forum.idx_forum_draft_forum;
DROP INDEX IF EXISTS forum.idx_forum_draft_topic;
ALTER TABLE forum.drafts ADD CONSTRAINT uq_forum_draft_topic UNIQUE (user_id, topic_id);
ALTER TABLE forum.drafts ADD CONSTRAINT uq_forum_draft_forum UNIQUE (user_id, forum_id);

-- ── Reports ──────────────────────────────────────────────────────────────────
-- "One open report per (post, reporter)" is conditioned on a VALUE (status =
-- 'open'), not on a NULL, so it cannot become a plain UNIQUE without forbidding
-- a fresh report after an old one was resolved. The guard moves into the
-- service (a pre-insert check, see ModerationService::report_post); the DB keeps
-- only a lookup index.
DROP INDEX IF EXISTS forum.uq_forum_reports_open;
CREATE INDEX IF NOT EXISTS idx_forum_reports_open ON forum.reports (post_id, reporter_id);
