-- Editable FAQ (phpBB-style): an admin-curated list of question/answer pairs
-- shown to every member on a dedicated page. `answer_md` is Markdown,
-- rendered client-side with the same sanitized renderer as post bodies.
-- `position` orders the list in both the admin console and the public page;
-- ties fall back to creation order.
CREATE TABLE forum.faq_entries (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question   TEXT NOT NULL,
    answer_md  TEXT NOT NULL,
    position   INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_faq_entries_position ON forum.faq_entries (position);
