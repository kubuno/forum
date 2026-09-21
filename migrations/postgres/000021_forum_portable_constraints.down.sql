DROP INDEX IF EXISTS forum.idx_forum_reports_open;
CREATE UNIQUE INDEX IF NOT EXISTS uq_forum_reports_open
    ON forum.reports (post_id, reporter_id) WHERE status = 'open';

ALTER TABLE forum.drafts DROP CONSTRAINT IF EXISTS uq_forum_draft_forum;
ALTER TABLE forum.drafts DROP CONSTRAINT IF EXISTS uq_forum_draft_topic;
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_draft_forum ON forum.drafts(user_id, forum_id) WHERE forum_id IS NOT NULL AND topic_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_draft_topic ON forum.drafts(user_id, topic_id) WHERE topic_id IS NOT NULL;

ALTER TABLE forum.subscriptions DROP CONSTRAINT IF EXISTS uq_forum_sub_forum;
ALTER TABLE forum.subscriptions DROP CONSTRAINT IF EXISTS uq_forum_sub_topic;
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_sub_topic ON forum.subscriptions(user_id, topic_id) WHERE topic_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_sub_forum ON forum.subscriptions(user_id, forum_id) WHERE forum_id IS NOT NULL;
