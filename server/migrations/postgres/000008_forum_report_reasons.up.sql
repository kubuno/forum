-- Predefined report reasons (phpBB-style): a moderator-curated list of chips a
-- reporter picks from, instead of always typing free text. The free-text
-- `reason` column stays — a picked reason is a category, the free text an
-- optional comment on top of it — so `reason_id` is nullable and a report
-- created before this migration (or by a legacy client) still reads fine.
CREATE TABLE forum.report_reasons (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title       TEXT NOT NULL,
    description TEXT,
    position    INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE forum.reports
    ADD COLUMN IF NOT EXISTS reason_id UUID REFERENCES forum.report_reasons(id) ON DELETE SET NULL;

-- A handful of sensible defaults so a fresh instance is not stuck with an
-- empty chip list; the admin can rename, reorder or delete them freely.
INSERT INTO forum.report_reasons (title, description, position) VALUES
    ('Spam',                  'Publicité, lien non sollicité ou contenu répétitif.', 0),
    ('Contenu inapproprié',   'Contenu choquant, illégal ou contraire aux règles.',  1),
    ('Hors-sujet',            'Ne concerne pas le sujet ou le forum.',               2),
    ('Harcèlement',           'Attaque personnelle, intimidation ou insulte.',       3);
