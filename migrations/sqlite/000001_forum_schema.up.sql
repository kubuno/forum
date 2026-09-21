-- SQLite — `forum` is an ATTACHed database file, attached on every pooled
-- connection by kubuno-db, so the qualified names below resolve as they do on
-- the other two engines. This single file declares the FINAL shape the
-- PostgreSQL side reached across its 000001..000022 migrations.
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BLOB, TIMESTAMPTZ -> TEXT (`%F %T%.f`, UTC), as sqlx encodes them.
--   * No DEFAULT on `id`: SQLite has no UUID generator; the process supplies it.
--   * JSONB -> TEXT (read back with `#[sqlx(json)]`).
--   * Full-text search is the normalized-column form (title_norm / body_norm,
--     filled in Rust): no tsvector, no unaccent.
--   * `key` is renamed to `field_key` (kept portable with the MySQL side).
--   * Foreign-key REFERENCES are unqualified (SQLite assumes the same database);
--     kubuno-db enables `PRAGMA foreign_keys`, so CASCADE deletes fire.
--   * updated_at is bumped by an AFTER UPDATE trigger per table (recursive
--     triggers are off by default, so the trigger's own write does not re-fire).

CREATE TABLE forum.categories (
    id          BLOB    NOT NULL PRIMARY KEY,
    name        TEXT    NOT NULL,
    description TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_cat_position ON categories(position);

CREATE TABLE forum.forums (
    id                BLOB    NOT NULL PRIMARY KEY,
    category_id       BLOB    NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    parent_forum_id   BLOB    REFERENCES forums(id) ON DELETE CASCADE,
    name              TEXT    NOT NULL,
    description       TEXT,
    position          INTEGER NOT NULL DEFAULT 0,
    is_locked         INTEGER NOT NULL DEFAULT 0,
    topic_count       INTEGER NOT NULL DEFAULT 0,
    post_count        INTEGER NOT NULL DEFAULT 0,
    last_post_id      BLOB,
    last_post_at      TEXT,
    last_post_user_id BLOB,
    color             TEXT,
    icon              TEXT,
    is_readonly       INTEGER NOT NULL DEFAULT 0,
    rules_md          TEXT,
    created_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_forum_category ON forums(category_id, position);
CREATE INDEX forum.idx_forum_forum_parent   ON forums(parent_forum_id) WHERE parent_forum_id IS NOT NULL;

CREATE TABLE forum.topics (
    id                BLOB    NOT NULL PRIMARY KEY,
    forum_id          BLOB    NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    author_id         BLOB    NOT NULL,
    title             TEXT    NOT NULL,
    slug              TEXT    NOT NULL,
    topic_type        TEXT    NOT NULL DEFAULT 'normal'
                          CHECK (topic_type IN ('normal', 'sticky', 'announcement', 'global')),
    is_locked         INTEGER NOT NULL DEFAULT 0,
    is_approved       INTEGER NOT NULL DEFAULT 1,
    view_count        INTEGER NOT NULL DEFAULT 0,
    reply_count       INTEGER NOT NULL DEFAULT 0,
    first_post_id     BLOB,
    last_post_id      BLOB,
    last_post_at      TEXT,
    last_post_user_id BLOB,
    is_solved         INTEGER NOT NULL DEFAULT 0,
    solution_post_id  BLOB,
    is_question       INTEGER NOT NULL DEFAULT 0,
    prefix            TEXT,
    approved_at       TEXT,
    approved_by       BLOB,
    is_deleted        INTEGER NOT NULL DEFAULT 0,
    deleted_at        TEXT,
    deleted_by        BLOB,
    delete_reason     TEXT,
    title_norm        TEXT    NOT NULL DEFAULT '',
    created_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_topic_forum      ON topics(forum_id, last_post_at);
CREATE INDEX forum.idx_forum_topic_author     ON topics(author_id);
CREATE INDEX forum.idx_forum_topic_slug       ON topics(slug);
CREATE INDEX forum.idx_forum_topics_pending   ON topics(created_at) WHERE is_approved = 0;
CREATE INDEX forum.idx_forum_topic_deleted    ON topics(forum_id) WHERE is_deleted = 1;
CREATE INDEX forum.idx_forum_topic_title_norm ON topics(title_norm);

CREATE TABLE forum.posts (
    id                BLOB    NOT NULL PRIMARY KEY,
    topic_id          BLOB    NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    forum_id          BLOB    NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    author_id         BLOB    NOT NULL,
    body_md           TEXT    NOT NULL,
    reply_to_post_id  BLOB    REFERENCES posts(id) ON DELETE SET NULL,
    is_first_post     INTEGER NOT NULL DEFAULT 0,
    is_approved       INTEGER NOT NULL DEFAULT 1,
    edited_at         TEXT,
    edited_by         BLOB,
    edit_reason       TEXT,
    edit_count        INTEGER NOT NULL DEFAULT 0,
    is_deleted        INTEGER NOT NULL DEFAULT 0,
    deleted_at        TEXT,
    deleted_by        BLOB,
    like_count        INTEGER NOT NULL DEFAULT 0,
    approved_at       TEXT,
    approved_by       BLOB,
    body_norm         TEXT    NOT NULL DEFAULT '',
    created_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_post_topic     ON posts(topic_id, created_at);
CREATE INDEX forum.idx_forum_post_forum     ON posts(forum_id);
CREATE INDEX forum.idx_forum_post_author    ON posts(author_id);
CREATE INDEX forum.idx_forum_post_deleted   ON posts(topic_id) WHERE is_deleted = 1;
CREATE INDEX forum.idx_forum_posts_pending  ON posts(created_at) WHERE is_approved = 0;
CREATE INDEX forum.idx_forum_post_body_norm ON posts(body_norm);

CREATE TABLE forum.attachments (
    id         BLOB    NOT NULL PRIMARY KEY,
    post_id    BLOB    NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    file_id    BLOB,
    filename   TEXT    NOT NULL,
    mime_type  TEXT,
    size_bytes BIGINT,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_attach_post ON attachments(post_id);

CREATE TABLE forum.moderators (
    forum_id   BLOB NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    user_id    BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (forum_id, user_id)
);
CREATE INDEX forum.idx_forum_mod_user ON moderators(user_id);

CREATE TABLE forum.report_reasons (
    id          BLOB    NOT NULL PRIMARY KEY,
    title       TEXT    NOT NULL,
    description TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.reports (
    id          BLOB    NOT NULL PRIMARY KEY,
    post_id     BLOB    NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    reporter_id BLOB    NOT NULL,
    reason      TEXT    NOT NULL,
    status      TEXT    NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'resolved', 'rejected')),
    handled_by  BLOB,
    handled_at  TEXT,
    reason_id   BLOB    REFERENCES report_reasons(id) ON DELETE SET NULL,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_report_status ON reports(status, created_at);
CREATE INDEX forum.idx_forum_report_post   ON reports(post_id);
CREATE INDEX forum.idx_forum_reports_open  ON reports(post_id, reporter_id);

CREATE TABLE forum.subscriptions (
    id         BLOB NOT NULL PRIMARY KEY,
    user_id    BLOB NOT NULL,
    topic_id   BLOB REFERENCES topics(id) ON DELETE CASCADE,
    forum_id   BLOB REFERENCES forums(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (user_id, topic_id),
    UNIQUE (user_id, forum_id)
);

CREATE TABLE forum.read_markers (
    user_id           BLOB NOT NULL,
    topic_id          BLOB NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    last_read_post_id BLOB,
    read_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (user_id, topic_id)
);

CREATE TABLE forum.ranks (
    id         BLOB    NOT NULL PRIMARY KEY,
    title      TEXT    NOT NULL,
    min_posts  INTEGER NOT NULL DEFAULT 0,
    is_special INTEGER NOT NULL DEFAULT 0,
    badge      TEXT,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_rank_minposts ON ranks(min_posts);

CREATE TABLE forum.user_profiles (
    user_id        BLOB    NOT NULL PRIMARY KEY,
    post_count     INTEGER NOT NULL DEFAULT 0,
    rank_id        BLOB    REFERENCES ranks(id) ON DELETE SET NULL,
    signature_md   TEXT,
    bio_md         TEXT,
    location       TEXT,
    website        TEXT,
    custom_title   TEXT,
    likes_received INTEGER NOT NULL DEFAULT 0,
    likes_given    INTEGER NOT NULL DEFAULT 0,
    topic_count    INTEGER NOT NULL DEFAULT 0,
    last_seen_at   TEXT,
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.permissions (
    id         BLOB    NOT NULL PRIMARY KEY,
    forum_id   BLOB    NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    role       TEXT    NOT NULL CHECK (role IN ('guest', 'user', 'moderator')),
    can_view   INTEGER NOT NULL DEFAULT 1,
    can_post   INTEGER NOT NULL DEFAULT 1,
    can_reply  INTEGER NOT NULL DEFAULT 1,
    can_attach INTEGER NOT NULL DEFAULT 1,
    UNIQUE (forum_id, role)
);

CREATE TABLE forum.reactions (
    id         BLOB    NOT NULL PRIMARY KEY,
    post_id    BLOB    NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id    BLOB    NOT NULL,
    emoji      TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (post_id, user_id, emoji)
);
CREATE INDEX forum.idx_forum_react_post ON reactions(post_id);
CREATE INDEX forum.idx_forum_react_user ON reactions(user_id);

CREATE TABLE forum.bookmarks (
    user_id    BLOB NOT NULL,
    topic_id   BLOB NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (user_id, topic_id)
);
CREATE INDEX forum.idx_forum_bookmark_user ON bookmarks(user_id, created_at);

CREATE TABLE forum.notifications (
    id              BLOB    NOT NULL PRIMARY KEY,
    user_id         BLOB    NOT NULL,
    kind            TEXT    NOT NULL,
    actor_id        BLOB,
    topic_id        BLOB    REFERENCES topics(id) ON DELETE CASCADE,
    post_id         BLOB    REFERENCES posts(id)  ON DELETE CASCADE,
    extra           TEXT,
    is_read         INTEGER NOT NULL DEFAULT 0,
    responder_count INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_notif_user   ON notifications(user_id, created_at);
CREATE INDEX forum.idx_forum_notif_unread ON notifications(user_id) WHERE is_read = 0;
CREATE INDEX forum.idx_forum_notif_fold   ON notifications(user_id, kind, topic_id) WHERE is_read = 0;

CREATE TABLE forum.drafts (
    id         BLOB NOT NULL PRIMARY KEY,
    user_id    BLOB NOT NULL,
    forum_id   BLOB REFERENCES forums(id) ON DELETE CASCADE,
    topic_id   BLOB REFERENCES topics(id) ON DELETE CASCADE,
    title      TEXT,
    body_md    TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (user_id, topic_id),
    UNIQUE (user_id, forum_id)
);

CREATE TABLE forum.tags (
    id         BLOB NOT NULL PRIMARY KEY,
    name       TEXT NOT NULL,
    slug       TEXT NOT NULL UNIQUE,
    color      TEXT NOT NULL DEFAULT '#0d9488',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.topic_tags (
    topic_id BLOB NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    tag_id   BLOB NOT NULL REFERENCES tags(id)   ON DELETE CASCADE,
    PRIMARY KEY (topic_id, tag_id)
);
CREATE INDEX forum.idx_forum_topictag_tag ON topic_tags(tag_id);

CREATE TABLE forum.polls (
    id          BLOB    NOT NULL PRIMARY KEY,
    topic_id    BLOB    NOT NULL UNIQUE REFERENCES topics(id) ON DELETE CASCADE,
    question    TEXT    NOT NULL,
    is_multiple INTEGER NOT NULL DEFAULT 0,
    closes_at   TEXT,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.poll_options (
    id       BLOB    NOT NULL PRIMARY KEY,
    poll_id  BLOB    NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    text     TEXT    NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX forum.idx_forum_pollopt_poll ON poll_options(poll_id, position);

CREATE TABLE forum.poll_votes (
    poll_id    BLOB NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    option_id  BLOB NOT NULL REFERENCES poll_options(id) ON DELETE CASCADE,
    user_id    BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (poll_id, option_id, user_id)
);
CREATE INDEX forum.idx_forum_pollvote_user ON poll_votes(poll_id, user_id);

CREATE TABLE forum.mod_log (
    id             INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    moderator_id   BLOB    NOT NULL,
    action         TEXT    NOT NULL,
    forum_id       BLOB,
    topic_id       BLOB,
    post_id        BLOB,
    target_user_id BLOB,
    details        TEXT,
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_modlog_created ON mod_log(created_at);

CREATE TABLE forum.user_warnings (
    id           BLOB NOT NULL PRIMARY KEY,
    user_id      BLOB NOT NULL,
    moderator_id BLOB NOT NULL,
    reason       TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_warn_user ON user_warnings(user_id, created_at);

CREATE TABLE forum.user_bans (
    user_id    BLOB NOT NULL PRIMARY KEY,
    banned_by  BLOB NOT NULL,
    reason     TEXT,
    until      TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.mod_notes (
    id             BLOB NOT NULL PRIMARY KEY,
    author_id      BLOB NOT NULL,
    target_user_id BLOB,
    topic_id       BLOB,
    post_id        BLOB,
    body           TEXT NOT NULL,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_modnote_user ON mod_notes(target_user_id, created_at);

CREATE TABLE forum.online (
    user_id      BLOB NOT NULL PRIMARY KEY,
    last_seen_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    path         TEXT
);
CREATE INDEX forum.idx_forum_online_seen ON online(last_seen_at);

CREATE TABLE forum.post_revisions (
    id          BLOB NOT NULL PRIMARY KEY,
    post_id     BLOB NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    body_md     TEXT NOT NULL,
    edited_by   BLOB,
    edit_reason TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_post_revisions_post ON post_revisions(post_id, created_at);

CREATE TABLE forum.censored_words (
    id          BLOB NOT NULL PRIMARY KEY,
    pattern     TEXT NOT NULL,
    replacement TEXT NOT NULL DEFAULT '***',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.pm_threads (
    id              BLOB NOT NULL PRIMARY KEY,
    subject         TEXT,
    created_by      BLOB NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    last_message_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.pm_participants (
    thread_id    BLOB    NOT NULL REFERENCES pm_threads(id) ON DELETE CASCADE,
    user_id      BLOB    NOT NULL,
    last_read_at TEXT,
    deleted      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (thread_id, user_id)
);
CREATE INDEX forum.idx_forum_pm_participants_user ON pm_participants(user_id);

CREATE TABLE forum.pm_messages (
    id         BLOB NOT NULL PRIMARY KEY,
    thread_id  BLOB NOT NULL REFERENCES pm_threads(id) ON DELETE CASCADE,
    sender_id  BLOB NOT NULL,
    body_md    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_forum_pm_messages_thread ON pm_messages(thread_id, created_at);

CREATE TABLE forum.pm_blocks (
    user_id         BLOB NOT NULL,
    blocked_user_id BLOB NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (user_id, blocked_user_id)
);

CREATE TABLE forum.profile_fields (
    id            BLOB    NOT NULL PRIMARY KEY,
    field_key     TEXT    NOT NULL UNIQUE,
    label         TEXT    NOT NULL,
    field_type    TEXT    NOT NULL
                      CHECK (field_type IN ('text', 'textarea', 'bool', 'url', 'date', 'dropdown')),
    options       TEXT,
    position      INTEGER NOT NULL DEFAULT 0,
    visibility    TEXT    NOT NULL DEFAULT 'public'
                      CHECK (visibility IN ('public', 'registered')),
    show_on_posts INTEGER NOT NULL DEFAULT 0,
    required      INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.profile_field_values (
    user_id  BLOB NOT NULL,
    field_id BLOB NOT NULL REFERENCES profile_fields(id) ON DELETE CASCADE,
    value    TEXT NOT NULL,
    PRIMARY KEY (user_id, field_id)
);
CREATE INDEX forum.idx_forum_profile_field_values_field ON profile_field_values(field_id);

CREATE TABLE forum.ignored_users (
    user_id         BLOB NOT NULL,
    ignored_user_id BLOB NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (user_id, ignored_user_id),
    CHECK (user_id <> ignored_user_id)
);
CREATE INDEX forum.idx_forum_ignored_users_user ON ignored_users(user_id);

CREATE TABLE forum.feed_tokens (
    token        TEXT NOT NULL PRIMARY KEY,
    user_id      BLOB NOT NULL,
    label        TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    last_used_at TEXT
);
CREATE INDEX forum.idx_forum_feed_tokens_user ON feed_tokens(user_id);

CREATE TABLE forum.faq_entries (
    id         BLOB    NOT NULL PRIMARY KEY,
    question   TEXT    NOT NULL,
    answer_md  TEXT    NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX forum.idx_faq_entries_position ON faq_entries(position);

CREATE TABLE forum.ip_bans (
    id         BLOB NOT NULL PRIMARY KEY,
    value      TEXT NOT NULL UNIQUE,
    reason     TEXT,
    banned_by  BLOB NOT NULL,
    until      TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.email_bans (
    id         BLOB NOT NULL PRIMARY KEY,
    email      TEXT NOT NULL UNIQUE,
    reason     TEXT,
    banned_by  BLOB NOT NULL,
    until      TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE forum.group_permissions (
    id         BLOB    NOT NULL PRIMARY KEY,
    forum_id   BLOB    NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    group_id   BLOB    NOT NULL,
    can_view   INTEGER NOT NULL DEFAULT 0,
    can_post   INTEGER NOT NULL DEFAULT 0,
    can_reply  INTEGER NOT NULL DEFAULT 0,
    can_attach INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (forum_id, group_id)
);
CREATE INDEX forum.idx_forum_group_permissions_group ON group_permissions(group_id);

-- updated_at maintenance (the PostgreSQL BEFORE UPDATE trigger's portable form).
-- Recursive triggers are OFF by default in SQLite, so the trigger's own UPDATE
-- does not re-fire it.
CREATE TRIGGER forum.categories_updated_at    AFTER UPDATE ON categories    FOR EACH ROW BEGIN UPDATE categories    SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id; END;
CREATE TRIGGER forum.forums_updated_at        AFTER UPDATE ON forums        FOR EACH ROW BEGIN UPDATE forums        SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id; END;
CREATE TRIGGER forum.topics_updated_at        AFTER UPDATE ON topics        FOR EACH ROW BEGIN UPDATE topics        SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id; END;
CREATE TRIGGER forum.posts_updated_at         AFTER UPDATE ON posts         FOR EACH ROW BEGIN UPDATE posts         SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id; END;
CREATE TRIGGER forum.user_profiles_updated_at AFTER UPDATE ON user_profiles FOR EACH ROW BEGIN UPDATE user_profiles SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE user_id = NEW.user_id; END;
CREATE TRIGGER forum.drafts_updated_at        AFTER UPDATE ON drafts        FOR EACH ROW BEGIN UPDATE drafts        SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id; END;
CREATE TRIGGER forum.faq_entries_updated_at   AFTER UPDATE ON faq_entries   FOR EACH ROW BEGIN UPDATE faq_entries   SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id; END;
