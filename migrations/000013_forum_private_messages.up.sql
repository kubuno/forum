-- Private messages (PM): modern 1:1 / small-group conversations, entirely
-- separate from the phpBB-style boards above — no folders, no per-message
-- rules engine. A thread is a set of participants sharing an ordered list of
-- messages. Every read of a thread or its messages MUST be filtered by
-- `forum.pm_participants` membership (see `services/pm_service.rs`) — a
-- non-participant gets 404, never 403, never a peek at the content.
CREATE TABLE forum.pm_threads (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject         TEXT,
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One row per (thread, member). `last_read_at` drives the unread badge: a
-- thread counts as unread for a participant when it holds a message from
-- someone else newer than their `last_read_at` (or they never read it at
-- all). `deleted` is a per-participant hide — the other side still sees the
-- thread; the thread and its messages are only actually dropped once every
-- participant has hidden it (see `PmService::delete_for_user`), via
-- `ON DELETE CASCADE` from `pm_threads`.
CREATE TABLE forum.pm_participants (
    thread_id    UUID NOT NULL REFERENCES forum.pm_threads(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL,
    last_read_at TIMESTAMPTZ,
    deleted      BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (thread_id, user_id)
);
CREATE INDEX idx_forum_pm_participants_user ON forum.pm_participants(user_id);

CREATE TABLE forum.pm_messages (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id  UUID NOT NULL REFERENCES forum.pm_threads(id) ON DELETE CASCADE,
    sender_id  UUID NOT NULL,
    body_md    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_pm_messages_thread ON forum.pm_messages(thread_id, created_at);

-- One-directional block: `user_id` no longer wants to receive messages from
-- `blocked_user_id`. Checked server-side before a thread is created or a
-- message is sent (`PmService::assert_not_blocked`) — never enforced only on
-- the client.
CREATE TABLE forum.pm_blocks (
    user_id         UUID NOT NULL,
    blocked_user_id UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, blocked_user_id)
);
