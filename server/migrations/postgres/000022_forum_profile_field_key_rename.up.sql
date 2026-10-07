-- `key` is a reserved word in MySQL and cannot appear unquoted in a shared
-- query string, so the column is renamed to a portable identifier. The API and
-- JSON keep the name `key` through a `#[sqlx(rename = "field_key")]` on the
-- model and the request DTOs, which are untouched.
ALTER TABLE forum.profile_fields RENAME COLUMN key TO field_key;
