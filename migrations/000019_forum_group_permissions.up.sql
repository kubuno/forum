-- Per-GROUP forum permissions (phpBB-style). Complements the role-based
-- `forum.permissions` (guest/user/moderator): a row here GRANTS additional
-- access to members of a core user group, letting an admin open an otherwise
-- restricted forum to specific groups. The semantics are strictly ADDITIVE —
-- a group grant can only widen access, never take it away — so it can never
-- silently hide a forum that the role rules already made visible.
--
-- `group_id` is a core user-group UUID (core.user_groups.id); it lives in the
-- core schema, so there is no cross-schema foreign key — the admin UI only ever
-- offers groups the core reports as existing.
CREATE TABLE forum.group_permissions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    forum_id   UUID NOT NULL REFERENCES forum.forums(id) ON DELETE CASCADE,
    group_id   UUID NOT NULL,
    can_view   BOOLEAN NOT NULL DEFAULT FALSE,
    can_post   BOOLEAN NOT NULL DEFAULT FALSE,
    can_reply  BOOLEAN NOT NULL DEFAULT FALSE,
    can_attach BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (forum_id, group_id)
);
CREATE INDEX idx_forum_group_permissions_forum ON forum.group_permissions(forum_id);
CREATE INDEX idx_forum_group_permissions_group ON forum.group_permissions(group_id);
