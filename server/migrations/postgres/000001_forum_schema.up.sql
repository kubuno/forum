-- Forum module — schema `forum` (categories → forums → topics → posts), inspired by phpBB.
-- The schema is created by main.rs; migrations run with search_path = forum,public.

-- ── Trigger function ────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION forum.set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ── Categories (top-level sections) ─────────────────────────────────────────────

CREATE TABLE forum.categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    description TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_cat_position ON forum.categories(position);

-- ── Forums and sub-forums (recursive) ───────────────────────────────────────────

CREATE TABLE forum.forums (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id       UUID NOT NULL REFERENCES forum.categories(id) ON DELETE CASCADE,
    parent_forum_id   UUID REFERENCES forum.forums(id) ON DELETE CASCADE,
    name              VARCHAR(255) NOT NULL,
    description       TEXT,
    position          INTEGER NOT NULL DEFAULT 0,
    is_locked         BOOLEAN NOT NULL DEFAULT FALSE,
    -- Denormalised counters maintained transactionally by the services.
    topic_count       INTEGER NOT NULL DEFAULT 0,
    post_count        INTEGER NOT NULL DEFAULT 0,
    -- Last post pointers (plain UUIDs, no FK to avoid circular references).
    last_post_id      UUID,
    last_post_at      TIMESTAMPTZ,
    last_post_user_id UUID,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_forum_category ON forum.forums(category_id, position);
CREATE INDEX idx_forum_forum_parent   ON forum.forums(parent_forum_id) WHERE parent_forum_id IS NOT NULL;

-- ── Topics (threads) ────────────────────────────────────────────────────────────

CREATE TABLE forum.topics (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    forum_id          UUID NOT NULL REFERENCES forum.forums(id) ON DELETE CASCADE,
    author_id         UUID NOT NULL,
    title             VARCHAR(500) NOT NULL,
    slug              VARCHAR(540) NOT NULL,
    topic_type        VARCHAR(20) NOT NULL DEFAULT 'normal'
                          CHECK (topic_type IN ('normal', 'sticky', 'announcement', 'global')),
    is_locked         BOOLEAN NOT NULL DEFAULT FALSE,
    is_approved       BOOLEAN NOT NULL DEFAULT TRUE,
    view_count        INTEGER NOT NULL DEFAULT 0,
    reply_count       INTEGER NOT NULL DEFAULT 0,
    first_post_id     UUID,
    last_post_id      UUID,
    last_post_at      TIMESTAMPTZ,
    last_post_user_id UUID,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_topic_forum  ON forum.topics(forum_id, topic_type DESC, last_post_at DESC NULLS LAST);
CREATE INDEX idx_forum_topic_author ON forum.topics(author_id);
CREATE INDEX idx_forum_topic_slug   ON forum.topics(slug);

-- ── Posts (messages, Markdown) ──────────────────────────────────────────────────

CREATE TABLE forum.posts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    topic_id          UUID NOT NULL REFERENCES forum.topics(id) ON DELETE CASCADE,
    -- Denormalised forum_id for search and permission checks.
    forum_id          UUID NOT NULL REFERENCES forum.forums(id) ON DELETE CASCADE,
    author_id         UUID NOT NULL,
    body_md           TEXT NOT NULL,
    reply_to_post_id  UUID REFERENCES forum.posts(id) ON DELETE SET NULL,
    is_first_post     BOOLEAN NOT NULL DEFAULT FALSE,
    is_approved       BOOLEAN NOT NULL DEFAULT TRUE,
    edited_at         TIMESTAMPTZ,
    edited_by         UUID,
    edit_reason       VARCHAR(500),
    edit_count        INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_post_topic  ON forum.posts(topic_id, created_at);
CREATE INDEX idx_forum_post_forum  ON forum.posts(forum_id);
CREATE INDEX idx_forum_post_author ON forum.posts(author_id);

-- ── Attachments (file references in the drive module) ────────────────────────────

CREATE TABLE forum.attachments (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id    UUID NOT NULL REFERENCES forum.posts(id) ON DELETE CASCADE,
    file_id    UUID,
    filename   VARCHAR(500) NOT NULL,
    mime_type  VARCHAR(255),
    size_bytes BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_attach_post ON forum.attachments(post_id);

-- ── Moderators (per forum) ───────────────────────────────────────────────────────

CREATE TABLE forum.moderators (
    forum_id   UUID NOT NULL REFERENCES forum.forums(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (forum_id, user_id)
);
CREATE INDEX idx_forum_mod_user ON forum.moderators(user_id);

-- ── Reports (flagged posts) ──────────────────────────────────────────────────────

CREATE TABLE forum.reports (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES forum.posts(id) ON DELETE CASCADE,
    reporter_id UUID NOT NULL,
    reason      TEXT NOT NULL,
    status      VARCHAR(20) NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'resolved', 'rejected')),
    handled_by  UUID,
    handled_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_report_status ON forum.reports(status, created_at);
CREATE INDEX idx_forum_report_post   ON forum.reports(post_id);

-- ── Subscriptions (watch a topic or a forum) ─────────────────────────────────────

CREATE TABLE forum.subscriptions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL,
    topic_id   UUID REFERENCES forum.topics(id) ON DELETE CASCADE,
    forum_id   UUID REFERENCES forum.forums(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sub_target_one CHECK (
        (topic_id IS NOT NULL AND forum_id IS NULL) OR
        (topic_id IS NULL AND forum_id IS NOT NULL)
    )
);
CREATE UNIQUE INDEX idx_forum_sub_topic ON forum.subscriptions(user_id, topic_id) WHERE topic_id IS NOT NULL;
CREATE UNIQUE INDEX idx_forum_sub_forum ON forum.subscriptions(user_id, forum_id) WHERE forum_id IS NOT NULL;

-- ── Read markers (unread tracking) ───────────────────────────────────────────────

CREATE TABLE forum.read_markers (
    user_id           UUID NOT NULL,
    topic_id          UUID NOT NULL REFERENCES forum.topics(id) ON DELETE CASCADE,
    last_read_post_id UUID,
    read_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, topic_id)
);

-- ── Ranks (phpBB-style, based on post count) ─────────────────────────────────────

CREATE TABLE forum.ranks (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title      VARCHAR(100) NOT NULL,
    min_posts  INTEGER NOT NULL DEFAULT 0,
    is_special BOOLEAN NOT NULL DEFAULT FALSE,
    badge      VARCHAR(40),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_forum_rank_minposts ON forum.ranks(min_posts);

-- ── Per-user forum profile ───────────────────────────────────────────────────────

CREATE TABLE forum.user_profiles (
    user_id      UUID PRIMARY KEY,
    post_count   INTEGER NOT NULL DEFAULT 0,
    rank_id      UUID REFERENCES forum.ranks(id) ON DELETE SET NULL,
    signature_md TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Per-forum permissions (simplified vs phpBB ACL) ──────────────────────────────

CREATE TABLE forum.permissions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    forum_id   UUID NOT NULL REFERENCES forum.forums(id) ON DELETE CASCADE,
    role       VARCHAR(20) NOT NULL CHECK (role IN ('guest', 'user', 'moderator')),
    can_view   BOOLEAN NOT NULL DEFAULT TRUE,
    can_post   BOOLEAN NOT NULL DEFAULT TRUE,
    can_reply  BOOLEAN NOT NULL DEFAULT TRUE,
    can_attach BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE (forum_id, role)
);

-- ── updated_at triggers ──────────────────────────────────────────────────────────

CREATE TRIGGER categories_updated_at    BEFORE UPDATE ON forum.categories    FOR EACH ROW EXECUTE FUNCTION forum.set_updated_at();
CREATE TRIGGER forums_updated_at        BEFORE UPDATE ON forum.forums        FOR EACH ROW EXECUTE FUNCTION forum.set_updated_at();
CREATE TRIGGER topics_updated_at        BEFORE UPDATE ON forum.topics        FOR EACH ROW EXECUTE FUNCTION forum.set_updated_at();
CREATE TRIGGER posts_updated_at         BEFORE UPDATE ON forum.posts         FOR EACH ROW EXECUTE FUNCTION forum.set_updated_at();
CREATE TRIGGER user_profiles_updated_at BEFORE UPDATE ON forum.user_profiles FOR EACH ROW EXECUTE FUNCTION forum.set_updated_at();
