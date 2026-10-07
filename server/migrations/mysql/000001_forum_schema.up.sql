-- MySQL / MariaDB — the `forum` database is created by kubuno-db's schema setup
-- before the migrator runs, so there is no CREATE DATABASE here. This single
-- file declares the FINAL shape the PostgreSQL side reached across its
-- 000001..000022 migrations (features, approval queue, soft-delete, report
-- reasons, revisions, censor, private messages, profile fields, ignore list,
-- feed tokens, FAQ, IP/email bans, group permissions, normalized search).
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BINARY(16): what sqlx encodes a `uuid::Uuid` as on MySQL.
--   * No DEFAULT on `id`: MySQL has no gen_random_uuid() and no RETURNING, so
--     the process supplies every primary key.
--   * TIMESTAMPTZ -> DATETIME(6); every value written is UTC (the pool pins the
--     session time zone to +00:00).
--   * updated_at is maintained by ON UPDATE CURRENT_TIMESTAMP(6), which replaces
--     the PostgreSQL BEFORE UPDATE trigger.
--   * Full-text search is the normalized-column form (title_norm / body_norm,
--     filled in Rust): no tsvector, no GIN, no unaccent.
--   * Partial indexes (WHERE ...) become plain indexes (MySQL has none).
--   * Partial UNIQUE indexes on a nullable pair become a plain UNIQUE: all three
--     engines treat NULLs as distinct, so rows carrying a NULL never collide.
--   * BIGSERIAL -> BIGINT AUTO_INCREMENT.
--   * `key` is renamed to `field_key` (reserved word in MySQL).
--   * TEXT columns that carry a UNIQUE/PRIMARY KEY become VARCHAR(190) (MySQL
--     cannot index an unbounded TEXT without a prefix length).
--   * utf8mb4_bin so a UNIQUE key stays case- and accent-sensitive.

CREATE TABLE categories (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    description TEXT         NULL,
    position    INT          NOT NULL DEFAULT 0,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_cat_position ON categories(position);

CREATE TABLE forums (
    id                BINARY(16)   NOT NULL PRIMARY KEY,
    category_id       BINARY(16)   NOT NULL,
    parent_forum_id   BINARY(16)   NULL,
    name              VARCHAR(255) NOT NULL,
    description       TEXT         NULL,
    position          INT          NOT NULL DEFAULT 0,
    is_locked         BOOLEAN      NOT NULL DEFAULT FALSE,
    topic_count       INT          NOT NULL DEFAULT 0,
    post_count        INT          NOT NULL DEFAULT 0,
    last_post_id      BINARY(16)   NULL,
    last_post_at      DATETIME(6)  NULL,
    last_post_user_id BINARY(16)   NULL,
    color             VARCHAR(7)   NULL,
    icon              VARCHAR(40)  NULL,
    is_readonly       BOOLEAN      NOT NULL DEFAULT FALSE,
    rules_md          TEXT         NULL,
    created_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (category_id)     REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_forum_id) REFERENCES forums(id)     ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_forum_category ON forums(category_id, position);
CREATE INDEX idx_forum_forum_parent   ON forums(parent_forum_id);

CREATE TABLE topics (
    id                BINARY(16)   NOT NULL PRIMARY KEY,
    forum_id          BINARY(16)   NOT NULL,
    author_id         BINARY(16)   NOT NULL,
    title             VARCHAR(500) NOT NULL,
    slug              VARCHAR(540) NOT NULL,
    topic_type        VARCHAR(20)  NOT NULL DEFAULT 'normal'
                          CHECK (topic_type IN ('normal', 'sticky', 'announcement', 'global')),
    is_locked         BOOLEAN      NOT NULL DEFAULT FALSE,
    is_approved       BOOLEAN      NOT NULL DEFAULT TRUE,
    view_count        INT          NOT NULL DEFAULT 0,
    reply_count       INT          NOT NULL DEFAULT 0,
    first_post_id     BINARY(16)   NULL,
    last_post_id      BINARY(16)   NULL,
    last_post_at      DATETIME(6)  NULL,
    last_post_user_id BINARY(16)   NULL,
    is_solved         BOOLEAN      NOT NULL DEFAULT FALSE,
    solution_post_id  BINARY(16)   NULL,
    is_question       BOOLEAN      NOT NULL DEFAULT FALSE,
    prefix            VARCHAR(40)  NULL,
    approved_at       DATETIME(6)  NULL,
    approved_by       BINARY(16)   NULL,
    is_deleted        BOOLEAN      NOT NULL DEFAULT FALSE,
    deleted_at        DATETIME(6)  NULL,
    deleted_by        BINARY(16)   NULL,
    delete_reason     TEXT         NULL,
    title_norm        TEXT         NOT NULL,
    created_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (forum_id) REFERENCES forums(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_topic_forum      ON topics(forum_id, last_post_at);
CREATE INDEX idx_forum_topic_author     ON topics(author_id);
CREATE INDEX idx_forum_topic_slug       ON topics(slug);
CREATE INDEX idx_forum_topics_pending   ON topics(created_at);
CREATE INDEX idx_forum_topic_deleted    ON topics(forum_id);
CREATE INDEX idx_forum_topic_title_norm ON topics(title_norm(191));

CREATE TABLE posts (
    id                BINARY(16)   NOT NULL PRIMARY KEY,
    topic_id          BINARY(16)   NOT NULL,
    forum_id          BINARY(16)   NOT NULL,
    author_id         BINARY(16)   NOT NULL,
    body_md           TEXT         NOT NULL,
    reply_to_post_id  BINARY(16)   NULL,
    is_first_post     BOOLEAN      NOT NULL DEFAULT FALSE,
    is_approved       BOOLEAN      NOT NULL DEFAULT TRUE,
    edited_at         DATETIME(6)  NULL,
    edited_by         BINARY(16)   NULL,
    edit_reason       VARCHAR(500) NULL,
    edit_count        INT          NOT NULL DEFAULT 0,
    is_deleted        BOOLEAN      NOT NULL DEFAULT FALSE,
    deleted_at        DATETIME(6)  NULL,
    deleted_by        BINARY(16)   NULL,
    like_count        INT          NOT NULL DEFAULT 0,
    approved_at       DATETIME(6)  NULL,
    approved_by       BINARY(16)   NULL,
    body_norm         TEXT         NOT NULL,
    created_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at        DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (topic_id)         REFERENCES topics(id) ON DELETE CASCADE,
    FOREIGN KEY (forum_id)         REFERENCES forums(id) ON DELETE CASCADE,
    FOREIGN KEY (reply_to_post_id) REFERENCES posts(id)  ON DELETE SET NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_post_topic     ON posts(topic_id, created_at);
CREATE INDEX idx_forum_post_forum     ON posts(forum_id);
CREATE INDEX idx_forum_post_author    ON posts(author_id);
CREATE INDEX idx_forum_post_deleted   ON posts(topic_id);
CREATE INDEX idx_forum_posts_pending  ON posts(created_at);
CREATE INDEX idx_forum_post_body_norm ON posts(body_norm(191));

CREATE TABLE attachments (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    post_id    BINARY(16)   NOT NULL,
    file_id    BINARY(16)   NULL,
    filename   VARCHAR(500) NOT NULL,
    mime_type  VARCHAR(255) NULL,
    size_bytes BIGINT       NULL,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_attach_post ON attachments(post_id);

CREATE TABLE moderators (
    forum_id   BINARY(16)  NOT NULL,
    user_id    BINARY(16)  NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (forum_id, user_id),
    FOREIGN KEY (forum_id) REFERENCES forums(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_mod_user ON moderators(user_id);

CREATE TABLE report_reasons (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    title       TEXT        NOT NULL,
    description TEXT        NULL,
    position    INT         NOT NULL DEFAULT 0,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE reports (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    post_id     BINARY(16)  NOT NULL,
    reporter_id BINARY(16)  NOT NULL,
    reason      TEXT        NOT NULL,
    status      VARCHAR(20) NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'resolved', 'rejected')),
    handled_by  BINARY(16)  NULL,
    handled_at  DATETIME(6) NULL,
    reason_id   BINARY(16)  NULL,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (post_id)   REFERENCES posts(id)          ON DELETE CASCADE,
    FOREIGN KEY (reason_id) REFERENCES report_reasons(id) ON DELETE SET NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_report_status ON reports(status, created_at);
CREATE INDEX idx_forum_report_post   ON reports(post_id);
CREATE INDEX idx_forum_reports_open  ON reports(post_id, reporter_id);

CREATE TABLE subscriptions (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    user_id    BINARY(16)  NOT NULL,
    topic_id   BINARY(16)  NULL,
    forum_id   BINARY(16)  NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (user_id, topic_id),
    UNIQUE (user_id, forum_id),
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE,
    FOREIGN KEY (forum_id) REFERENCES forums(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE read_markers (
    user_id           BINARY(16)  NOT NULL,
    topic_id          BINARY(16)  NOT NULL,
    last_read_post_id BINARY(16)  NULL,
    read_at           DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (user_id, topic_id),
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE ranks (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    title      VARCHAR(100) NOT NULL,
    min_posts  INT          NOT NULL DEFAULT 0,
    is_special BOOLEAN      NOT NULL DEFAULT FALSE,
    badge      VARCHAR(40)  NULL,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_rank_minposts ON ranks(min_posts);

CREATE TABLE user_profiles (
    user_id        BINARY(16)   NOT NULL PRIMARY KEY,
    post_count     INT          NOT NULL DEFAULT 0,
    rank_id        BINARY(16)   NULL,
    signature_md   TEXT         NULL,
    bio_md         TEXT         NULL,
    location       VARCHAR(120) NULL,
    website        VARCHAR(300) NULL,
    custom_title   VARCHAR(120) NULL,
    likes_received INT          NOT NULL DEFAULT 0,
    likes_given    INT          NOT NULL DEFAULT 0,
    topic_count    INT          NOT NULL DEFAULT 0,
    last_seen_at   DATETIME(6)  NULL,
    created_at     DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at     DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (rank_id) REFERENCES ranks(id) ON DELETE SET NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE permissions (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    forum_id   BINARY(16)  NOT NULL,
    role       VARCHAR(20) NOT NULL CHECK (role IN ('guest', 'user', 'moderator')),
    can_view   BOOLEAN     NOT NULL DEFAULT TRUE,
    can_post   BOOLEAN     NOT NULL DEFAULT TRUE,
    can_reply  BOOLEAN     NOT NULL DEFAULT TRUE,
    can_attach BOOLEAN     NOT NULL DEFAULT TRUE,
    UNIQUE (forum_id, role),
    FOREIGN KEY (forum_id) REFERENCES forums(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE reactions (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    post_id    BINARY(16)  NOT NULL,
    user_id    BINARY(16)  NOT NULL,
    emoji      VARCHAR(16) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (post_id, user_id, emoji),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_react_post ON reactions(post_id);
CREATE INDEX idx_forum_react_user ON reactions(user_id);

CREATE TABLE bookmarks (
    user_id    BINARY(16)  NOT NULL,
    topic_id   BINARY(16)  NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (user_id, topic_id),
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_bookmark_user ON bookmarks(user_id, created_at);

CREATE TABLE notifications (
    id              BINARY(16)   NOT NULL PRIMARY KEY,
    user_id         BINARY(16)   NOT NULL,
    kind            VARCHAR(24)  NOT NULL,
    actor_id        BINARY(16)   NULL,
    topic_id        BINARY(16)   NULL,
    post_id         BINARY(16)   NULL,
    extra           VARCHAR(255) NULL,
    is_read         BOOLEAN      NOT NULL DEFAULT FALSE,
    responder_count INT          NOT NULL DEFAULT 1,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE,
    FOREIGN KEY (post_id)  REFERENCES posts(id)  ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_notif_user   ON notifications(user_id, created_at);
CREATE INDEX idx_forum_notif_unread ON notifications(user_id);
CREATE INDEX idx_forum_notif_fold   ON notifications(user_id, kind, topic_id);

CREATE TABLE drafts (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    user_id    BINARY(16)   NOT NULL,
    forum_id   BINARY(16)   NULL,
    topic_id   BINARY(16)   NULL,
    title      VARCHAR(500) NULL,
    body_md    TEXT         NOT NULL,
    updated_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE (user_id, topic_id),
    UNIQUE (user_id, forum_id),
    FOREIGN KEY (forum_id) REFERENCES forums(id) ON DELETE CASCADE,
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE tags (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    name       VARCHAR(60) NOT NULL,
    slug       VARCHAR(70) NOT NULL UNIQUE,
    color      VARCHAR(7)  NOT NULL DEFAULT '#0d9488',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE topic_tags (
    topic_id BINARY(16) NOT NULL,
    tag_id   BINARY(16) NOT NULL,
    PRIMARY KEY (topic_id, tag_id),
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id)   REFERENCES tags(id)   ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_topictag_tag ON topic_tags(tag_id);

CREATE TABLE polls (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    topic_id    BINARY(16)   NOT NULL UNIQUE,
    question    VARCHAR(500) NOT NULL,
    is_multiple BOOLEAN      NOT NULL DEFAULT FALSE,
    closes_at   DATETIME(6)  NULL,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE poll_options (
    id       BINARY(16)   NOT NULL PRIMARY KEY,
    poll_id  BINARY(16)   NOT NULL,
    text     VARCHAR(255) NOT NULL,
    position INT          NOT NULL DEFAULT 0,
    FOREIGN KEY (poll_id) REFERENCES polls(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_pollopt_poll ON poll_options(poll_id, position);

CREATE TABLE poll_votes (
    poll_id    BINARY(16)  NOT NULL,
    option_id  BINARY(16)  NOT NULL,
    user_id    BINARY(16)  NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (poll_id, option_id, user_id),
    FOREIGN KEY (poll_id)   REFERENCES polls(id)        ON DELETE CASCADE,
    FOREIGN KEY (option_id) REFERENCES poll_options(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_pollvote_user ON poll_votes(poll_id, user_id);

CREATE TABLE mod_log (
    id             BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    moderator_id   BINARY(16)   NOT NULL,
    action         VARCHAR(40)  NOT NULL,
    forum_id       BINARY(16)   NULL,
    topic_id       BINARY(16)   NULL,
    post_id        BINARY(16)   NULL,
    target_user_id BINARY(16)   NULL,
    details        VARCHAR(500) NULL,
    created_at     DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_modlog_created ON mod_log(created_at);

CREATE TABLE user_warnings (
    id           BINARY(16)  NOT NULL PRIMARY KEY,
    user_id      BINARY(16)  NOT NULL,
    moderator_id BINARY(16)  NOT NULL,
    reason       TEXT        NOT NULL,
    created_at   DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_warn_user ON user_warnings(user_id, created_at);

CREATE TABLE user_bans (
    user_id    BINARY(16)  NOT NULL PRIMARY KEY,
    banned_by  BINARY(16)  NOT NULL,
    reason     TEXT        NULL,
    until      DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE mod_notes (
    id             BINARY(16)  NOT NULL PRIMARY KEY,
    author_id      BINARY(16)  NOT NULL,
    target_user_id BINARY(16)  NULL,
    topic_id       BINARY(16)  NULL,
    post_id        BINARY(16)  NULL,
    body           TEXT        NOT NULL,
    created_at     DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_modnote_user ON mod_notes(target_user_id, created_at);

CREATE TABLE online (
    user_id      BINARY(16)   NOT NULL PRIMARY KEY,
    last_seen_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    path         VARCHAR(255) NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_online_seen ON online(last_seen_at);

CREATE TABLE post_revisions (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    post_id     BINARY(16)  NOT NULL,
    body_md     TEXT        NOT NULL,
    edited_by   BINARY(16)  NULL,
    edit_reason TEXT        NULL,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_post_revisions_post ON post_revisions(post_id, created_at);

CREATE TABLE censored_words (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    pattern     TEXT        NOT NULL,
    replacement TEXT        NOT NULL,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE pm_threads (
    id              BINARY(16)  NOT NULL PRIMARY KEY,
    subject         TEXT        NULL,
    created_by      BINARY(16)  NOT NULL,
    created_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    last_message_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE pm_participants (
    thread_id    BINARY(16)  NOT NULL,
    user_id      BINARY(16)  NOT NULL,
    last_read_at DATETIME(6) NULL,
    deleted      BOOLEAN     NOT NULL DEFAULT FALSE,
    PRIMARY KEY (thread_id, user_id),
    FOREIGN KEY (thread_id) REFERENCES pm_threads(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_pm_participants_user ON pm_participants(user_id);

CREATE TABLE pm_messages (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    thread_id  BINARY(16)  NOT NULL,
    sender_id  BINARY(16)  NOT NULL,
    body_md    TEXT        NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (thread_id) REFERENCES pm_threads(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_pm_messages_thread ON pm_messages(thread_id, created_at);

CREATE TABLE pm_blocks (
    user_id         BINARY(16)  NOT NULL,
    blocked_user_id BINARY(16)  NOT NULL,
    created_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (user_id, blocked_user_id)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE profile_fields (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    field_key     VARCHAR(190) NOT NULL UNIQUE,
    label         TEXT         NOT NULL,
    field_type    TEXT         NOT NULL
                      CHECK (field_type IN ('text', 'textarea', 'bool', 'url', 'date', 'dropdown')),
    options       JSON         NULL,
    position      INT          NOT NULL DEFAULT 0,
    visibility    TEXT         NOT NULL DEFAULT 'public'
                      CHECK (visibility IN ('public', 'registered')),
    show_on_posts BOOLEAN      NOT NULL DEFAULT FALSE,
    required      BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE profile_field_values (
    user_id  BINARY(16) NOT NULL,
    field_id BINARY(16) NOT NULL,
    value    TEXT       NOT NULL,
    PRIMARY KEY (user_id, field_id),
    FOREIGN KEY (field_id) REFERENCES profile_fields(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_profile_field_values_field ON profile_field_values(field_id);

CREATE TABLE ignored_users (
    user_id         BINARY(16)  NOT NULL,
    ignored_user_id BINARY(16)  NOT NULL,
    created_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (user_id, ignored_user_id),
    CHECK (user_id <> ignored_user_id)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_ignored_users_user ON ignored_users(user_id);

CREATE TABLE feed_tokens (
    token        VARCHAR(190) NOT NULL PRIMARY KEY,
    user_id      BINARY(16)   NOT NULL,
    label        TEXT         NULL,
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    last_used_at DATETIME(6)  NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_feed_tokens_user ON feed_tokens(user_id);

CREATE TABLE faq_entries (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    question   TEXT        NOT NULL,
    answer_md  TEXT        NOT NULL,
    position   INT         NOT NULL DEFAULT 0,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_faq_entries_position ON faq_entries(position);

CREATE TABLE ip_bans (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    value      VARCHAR(190) NOT NULL UNIQUE,
    reason     TEXT         NULL,
    banned_by  BINARY(16)   NOT NULL,
    until      DATETIME(6)  NULL,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE email_bans (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    email      VARCHAR(190) NOT NULL UNIQUE,
    reason     TEXT         NULL,
    banned_by  BINARY(16)   NOT NULL,
    until      DATETIME(6)  NULL,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE group_permissions (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    forum_id   BINARY(16)  NOT NULL,
    group_id   BINARY(16)  NOT NULL,
    can_view   BOOLEAN     NOT NULL DEFAULT FALSE,
    can_post   BOOLEAN     NOT NULL DEFAULT FALSE,
    can_reply  BOOLEAN     NOT NULL DEFAULT FALSE,
    can_attach BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (forum_id, group_id),
    FOREIGN KEY (forum_id) REFERENCES forums(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_forum_group_permissions_group ON group_permissions(group_id);
