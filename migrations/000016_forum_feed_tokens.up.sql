-- Personal RSS/Atom feed tokens. A member mints an unguessable token that
-- lets a feed reader fetch their own view of recent topics WITHOUT a login
-- (feed readers cannot carry a session). The token travels in the URL, exactly
-- like the calendar module's ICS feed and drive's public share links, and is
-- the ONLY credential the anonymous `/public/feeds/:token/...` routes accept.
-- Revoking a row instantly kills its feed URL. There is deliberately no
-- anonymous, token-less feed: this is a self-hosted workspace with no "guest"
-- role, so member-visible content is never exposed to the open internet.
CREATE TABLE forum.feed_tokens (
    token        TEXT PRIMARY KEY,
    user_id      UUID NOT NULL,
    label        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX idx_forum_feed_tokens_user ON forum.feed_tokens(user_id);
