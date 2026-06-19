DROP TABLE IF EXISTS forum.online;
DROP TABLE IF EXISTS forum.mod_notes;
DROP TABLE IF EXISTS forum.user_bans;
DROP TABLE IF EXISTS forum.user_warnings;
DROP TABLE IF EXISTS forum.mod_log;
DROP TABLE IF EXISTS forum.poll_votes;
DROP TABLE IF EXISTS forum.poll_options;
DROP TABLE IF EXISTS forum.polls;
DROP TABLE IF EXISTS forum.topic_tags;
DROP TABLE IF EXISTS forum.tags;
DROP TABLE IF EXISTS forum.drafts;
DROP TABLE IF EXISTS forum.notifications;
DROP TABLE IF EXISTS forum.bookmarks;
DROP TABLE IF EXISTS forum.reactions;

ALTER TABLE forum.user_profiles
    DROP COLUMN IF EXISTS bio_md, DROP COLUMN IF EXISTS location, DROP COLUMN IF EXISTS website,
    DROP COLUMN IF EXISTS custom_title, DROP COLUMN IF EXISTS likes_received,
    DROP COLUMN IF EXISTS likes_given, DROP COLUMN IF EXISTS topic_count, DROP COLUMN IF EXISTS last_seen_at;
ALTER TABLE forum.forums
    DROP COLUMN IF EXISTS color, DROP COLUMN IF EXISTS icon,
    DROP COLUMN IF EXISTS is_readonly, DROP COLUMN IF EXISTS rules_md;
ALTER TABLE forum.posts
    DROP COLUMN IF EXISTS is_deleted, DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS deleted_by, DROP COLUMN IF EXISTS like_count;
ALTER TABLE forum.topics
    DROP COLUMN IF EXISTS is_solved, DROP COLUMN IF EXISTS solution_post_id,
    DROP COLUMN IF EXISTS is_question, DROP COLUMN IF EXISTS prefix;
