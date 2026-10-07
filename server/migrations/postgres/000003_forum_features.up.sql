-- New feature surface for the forum: reactions, solved topics, bookmarks,
-- notifications, drafts, tags, polls, moderation tooling, richer profiles and
-- presence. Counters stay denormalised and maintained by the services.

-- ── Extra columns ────────────────────────────────────────────────────────────
ALTER TABLE forum.topics
    ADD COLUMN IF NOT EXISTS is_solved        BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS solution_post_id UUID,
    ADD COLUMN IF NOT EXISTS is_question      BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS prefix           VARCHAR(40);

ALTER TABLE forum.posts
    ADD COLUMN IF NOT EXISTS is_deleted  BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deleted_at  TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deleted_by  UUID,
    ADD COLUMN IF NOT EXISTS like_count  INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_forum_post_deleted ON forum.posts(topic_id) WHERE is_deleted = TRUE;

ALTER TABLE forum.forums
    ADD COLUMN IF NOT EXISTS color       VARCHAR(7),
    ADD COLUMN IF NOT EXISTS icon        VARCHAR(40),
    ADD COLUMN IF NOT EXISTS is_readonly BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS rules_md    TEXT;

ALTER TABLE forum.user_profiles
    ADD COLUMN IF NOT EXISTS bio_md         TEXT,
    ADD COLUMN IF NOT EXISTS location       VARCHAR(120),
    ADD COLUMN IF NOT EXISTS website        VARCHAR(300),
    ADD COLUMN IF NOT EXISTS custom_title   VARCHAR(120),
    ADD COLUMN IF NOT EXISTS likes_received INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS likes_given    INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS topic_count    INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS last_seen_at   TIMESTAMPTZ;

-- ── Reactions (emoji per post per user) ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.reactions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id    UUID NOT NULL REFERENCES forum.posts(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL,
    emoji      VARCHAR(16) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (post_id, user_id, emoji)
);
CREATE INDEX IF NOT EXISTS idx_forum_react_post ON forum.reactions(post_id);
CREATE INDEX IF NOT EXISTS idx_forum_react_user ON forum.reactions(user_id);

-- ── Bookmarks (saved topics) ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.bookmarks (
    user_id    UUID NOT NULL,
    topic_id   UUID NOT NULL REFERENCES forum.topics(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, topic_id)
);
CREATE INDEX IF NOT EXISTS idx_forum_bookmark_user ON forum.bookmarks(user_id, created_at DESC);

-- ── Notifications ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.notifications (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL,                       -- recipient
    kind       VARCHAR(24) NOT NULL,                -- reply | mention | reaction | solution | quote | topic
    actor_id   UUID,                                -- who triggered it
    topic_id   UUID REFERENCES forum.topics(id) ON DELETE CASCADE,
    post_id    UUID REFERENCES forum.posts(id)  ON DELETE CASCADE,
    extra      VARCHAR(255),
    is_read    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_forum_notif_user ON forum.notifications(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_forum_notif_unread ON forum.notifications(user_id) WHERE is_read = FALSE;

-- ── Drafts (autosaved composer content) ──────────────────────────────────────
-- One draft per (user, context): a forum (new topic) or a topic (reply).
CREATE TABLE IF NOT EXISTS forum.drafts (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL,
    forum_id   UUID REFERENCES forum.forums(id) ON DELETE CASCADE,
    topic_id   UUID REFERENCES forum.topics(id) ON DELETE CASCADE,
    title      VARCHAR(500),
    body_md    TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_draft_forum ON forum.drafts(user_id, forum_id) WHERE forum_id IS NOT NULL AND topic_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_draft_topic ON forum.drafts(user_id, topic_id) WHERE topic_id IS NOT NULL;

-- ── Tags ─────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.tags (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       VARCHAR(60) NOT NULL,
    slug       VARCHAR(70) NOT NULL UNIQUE,
    color      VARCHAR(7) NOT NULL DEFAULT '#0d9488',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS forum.topic_tags (
    topic_id UUID NOT NULL REFERENCES forum.topics(id) ON DELETE CASCADE,
    tag_id   UUID NOT NULL REFERENCES forum.tags(id)   ON DELETE CASCADE,
    PRIMARY KEY (topic_id, tag_id)
);
CREATE INDEX IF NOT EXISTS idx_forum_topictag_tag ON forum.topic_tags(tag_id);

-- ── Polls ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.polls (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id      UUID NOT NULL UNIQUE REFERENCES forum.topics(id) ON DELETE CASCADE,
    question      VARCHAR(500) NOT NULL,
    is_multiple   BOOLEAN NOT NULL DEFAULT FALSE,
    closes_at     TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS forum.poll_options (
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id  UUID NOT NULL REFERENCES forum.polls(id) ON DELETE CASCADE,
    text     VARCHAR(255) NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_forum_pollopt_poll ON forum.poll_options(poll_id, position);
CREATE TABLE IF NOT EXISTS forum.poll_votes (
    poll_id   UUID NOT NULL REFERENCES forum.polls(id) ON DELETE CASCADE,
    option_id UUID NOT NULL REFERENCES forum.poll_options(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (poll_id, option_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_forum_pollvote_user ON forum.poll_votes(poll_id, user_id);

-- ── Moderation tooling ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.mod_log (
    id             BIGSERIAL PRIMARY KEY,
    moderator_id   UUID NOT NULL,
    action         VARCHAR(40) NOT NULL,
    forum_id       UUID,
    topic_id       UUID,
    post_id        UUID,
    target_user_id UUID,
    details        VARCHAR(500),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_forum_modlog_created ON forum.mod_log(created_at DESC);

CREATE TABLE IF NOT EXISTS forum.user_warnings (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL,
    moderator_id UUID NOT NULL,
    reason       TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_forum_warn_user ON forum.user_warnings(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS forum.user_bans (
    user_id    UUID PRIMARY KEY,
    banned_by  UUID NOT NULL,
    reason     TEXT,
    until      TIMESTAMPTZ,                      -- NULL = permanent
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS forum.mod_notes (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id      UUID NOT NULL,
    target_user_id UUID,
    topic_id       UUID,
    post_id        UUID,
    body           TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_forum_modnote_user ON forum.mod_notes(target_user_id, created_at DESC);

-- ── Presence (who's online) ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS forum.online (
    user_id      UUID PRIMARY KEY,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    path         VARCHAR(255)
);
CREATE INDEX IF NOT EXISTS idx_forum_online_seen ON forum.online(last_seen_at DESC);
