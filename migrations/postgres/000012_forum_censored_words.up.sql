-- Admin-curated word censor (phpBB-style): a list of patterns substituted in
-- post bodies at render time, server-side — `CensorService::apply` runs
-- before a post is serialized, so a client can never bypass it by reading the
-- raw payload differently. `pattern` is a literal word/phrase (never a regex
-- fragment supplied by the admin): the service escapes it before compiling a
-- case-insensitive, word-boundary regex (see `services/censor_service.rs`).
CREATE TABLE forum.censored_words (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pattern     TEXT NOT NULL,
    replacement TEXT NOT NULL DEFAULT '***',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
