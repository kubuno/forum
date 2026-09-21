-- IP and email bans (phpBB-style): an admin-curated block list enforced
-- server-side on every authenticated request (see `services::ban_registry`
-- and `middleware::enforce_ban`), in addition to the existing per-account ban
-- (`forum.user_bans`). Matching is always an EXACT string comparison — no
-- CIDR, no wildcard — so the rule stays simple to audit and cannot silently
-- over-match.
CREATE TABLE forum.ip_bans (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    value      TEXT NOT NULL UNIQUE,             -- exact IP address, e.g. 203.0.113.4
    reason     TEXT,
    banned_by  UUID NOT NULL,
    until      TIMESTAMPTZ,                      -- NULL = permanent
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Email always stored lowercase (services::moderation_service normalizes
-- before every write), so a lookup never has to fold case at query time.
CREATE TABLE forum.email_bans (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email      TEXT NOT NULL UNIQUE,
    reason     TEXT,
    banned_by  UUID NOT NULL,
    until      TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
