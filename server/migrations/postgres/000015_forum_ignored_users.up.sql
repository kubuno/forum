-- Ignore list (phpBB-style "foes"/zebra): a member can ignore another member.
-- Purely a CLIENT-SIDE display preference — the backend keeps returning every
-- post regardless of this table; only the frontend folds an ignored author's
-- messages by default (see `services/ignore_service.rs`).
CREATE TABLE forum.ignored_users (
    user_id         UUID NOT NULL,
    ignored_user_id UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, ignored_user_id),
    CHECK (user_id <> ignored_user_id)
);
CREATE INDEX idx_forum_ignored_users_user ON forum.ignored_users(user_id);
