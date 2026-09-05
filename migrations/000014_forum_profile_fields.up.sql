-- Custom profile fields (phpBB-style "custom profile fields"): an admin
-- defines a small set of fields (text/textarea/bool/url/date/dropdown), each
-- member fills in their own answers, and the answers are shown on their
-- profile. This is a classic EAV pair: `profile_fields` holds the admin's
-- definitions, `profile_field_values` holds one row per (member, field).
--
-- Values are ALWAYS rendered as plain, escaped text (see
-- `ProfileFieldService::values_for_display` and the frontend `ProfilePage`) —
-- never as Markdown/HTML — so there is no injection surface here, unlike post
-- bodies. `url` values are further restricted to http(s) at write time (see
-- `ProfileFieldService::set_values`).
CREATE TABLE forum.profile_fields (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Stable machine name (e.g. "discord_handle"), never shown to members —
    -- `label` is what they see. Immutable-in-spirit but editable by an admin
    -- like any other column.
    key           TEXT NOT NULL UNIQUE,
    label         TEXT NOT NULL,
    field_type    TEXT NOT NULL CHECK (field_type IN ('text', 'textarea', 'bool', 'url', 'date', 'dropdown')),
    -- Only meaningful (and only ever populated) for `field_type = 'dropdown'`:
    -- a JSON array of the allowed option strings.
    options       JSONB,
    position      INT NOT NULL DEFAULT 0,
    visibility    TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'registered')),
    -- Whether this field's value is also surfaced next to a member's posts
    -- (phpBB-style), in addition to their profile page.
    show_on_posts BOOLEAN NOT NULL DEFAULT FALSE,
    required      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One member's answer to one field. `value` is always plain text — even for
-- `bool` ("true"/"false") and `date` (ISO "YYYY-MM-DD") — so the whole table
-- stays a single TEXT column regardless of `field_type`; the service layer
-- validates/parses according to the field's declared type.
CREATE TABLE forum.profile_field_values (
    user_id  UUID NOT NULL,
    field_id UUID NOT NULL REFERENCES forum.profile_fields(id) ON DELETE CASCADE,
    value    TEXT NOT NULL,
    PRIMARY KEY (user_id, field_id)
);
CREATE INDEX idx_forum_profile_field_values_field ON forum.profile_field_values(field_id);
