-- Move full-text search off PostgreSQL's `tsvector` / `to_tsvector` / `ts_rank`
-- and onto `kubuno_db::search`: post bodies and topic titles are reduced to
-- Snowball French stems and deaccented IN RUST at write time and stored in
-- plain `TEXT` columns, then a query is put through the same reduction and
-- matched with a portable `LIKE`. Because the stemming happens before any SQL,
-- the stored and searched tokens are byte-for-byte identical on PostgreSQL,
-- MySQL and SQLite. The MySQL and SQLite migrations declare the `*_norm`
-- columns from their CREATE TABLE; here the PostgreSQL tables shed their
-- `tsvector` machinery and gain the columns.
--
-- What is lost: `pg_trgm`'s typo tolerance (a `LIKE '%stem%'` needs the stem to
-- appear as a substring). Stemming still folds inflections and the normalizer
-- folds accents, so inflected and accented queries still match.

-- Retire the generated tsvector columns and their GIN indexes.
DROP INDEX IF EXISTS forum.idx_forum_post_search;
DROP INDEX IF EXISTS forum.idx_forum_topic_search;
ALTER TABLE forum.posts  DROP COLUMN IF EXISTS search_vector;
ALTER TABLE forum.topics DROP COLUMN IF EXISTS search_vector;

-- One normalized TEXT column per searchable text: the post body (weight B) and
-- the topic title (weight A). Existing rows get an empty string; every save
-- recomputes the column in Rust.
ALTER TABLE forum.posts  ADD COLUMN IF NOT EXISTS body_norm  TEXT NOT NULL DEFAULT '';
ALTER TABLE forum.topics ADD COLUMN IF NOT EXISTS title_norm TEXT NOT NULL DEFAULT '';

-- A plain B-tree is enough for a bound `LIKE '%stem%'`.
CREATE INDEX IF NOT EXISTS idx_forum_post_body_norm   ON forum.posts(body_norm);
CREATE INDEX IF NOT EXISTS idx_forum_topic_title_norm ON forum.topics(title_norm);
