-- Post edit history: every time a message is edited, the version being
-- overwritten is archived here first, so the author and moderators can see
-- what a message used to say. Purely additive; nothing here changes the
-- shape or behaviour of `forum.posts` itself.
CREATE TABLE IF NOT EXISTS forum.post_revisions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES forum.posts(id) ON DELETE CASCADE,
    body_md     TEXT NOT NULL,
    edited_by   UUID,
    edit_reason TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forum_post_revisions_post
    ON forum.post_revisions(post_id, created_at DESC);
