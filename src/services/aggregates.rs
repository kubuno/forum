//! Denormalised counter maintenance. Recomputing from the source rows after each
//! mutation keeps the cached aggregates correct without fragile incremental logic.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::errors::Result;

/// Build a URL-friendly slug from a title (ASCII, lowercase, hyphenated).
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    let trimmed = if trimmed.len() > 80 { trimmed[..80].trim_matches('-').to_string() } else { trimmed };
    if trimmed.is_empty() { "topic".to_string() } else { trimmed }
}

/// Recompute a topic's first/last post pointers and reply count.
///
/// Only APPROVED messages count. These aggregates are what the lists show —
/// "3 replies, last one an hour ago" — so counting a message still waiting in
/// the moderation queue would advertise content the reader cannot open, and
/// would leak that someone posted something a moderator has not released.
pub async fn recompute_topic(conn: &mut PgConnection, topic_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE forum.topics t SET
            reply_count       = GREATEST((SELECT COUNT(*) FROM forum.posts p WHERE p.topic_id = t.id AND p.is_approved AND NOT p.is_deleted) - 1, 0),
            first_post_id     = (SELECT p.id FROM forum.posts p WHERE p.topic_id = t.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at, p.id LIMIT 1),
            last_post_id      = (SELECT p.id FROM forum.posts p WHERE p.topic_id = t.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at DESC, p.id DESC LIMIT 1),
            last_post_at      = (SELECT p.created_at FROM forum.posts p WHERE p.topic_id = t.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at DESC, p.id DESC LIMIT 1),
            last_post_user_id = (SELECT p.author_id FROM forum.posts p WHERE p.topic_id = t.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at DESC, p.id DESC LIMIT 1)
         WHERE t.id = $1",
    )
    .bind(topic_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Recompute a forum's topic/post counts and last-post pointers. Approved rows
/// only, for the same reason as `recompute_topic`.
pub async fn recompute_forum(conn: &mut PgConnection, forum_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE forum.forums f SET
            topic_count       = (SELECT COUNT(*) FROM forum.topics t WHERE t.forum_id = f.id AND t.is_approved AND NOT t.is_deleted),
            post_count        = (SELECT COUNT(*) FROM forum.posts  p WHERE p.forum_id = f.id AND p.is_approved AND NOT p.is_deleted),
            last_post_id      = (SELECT p.id FROM forum.posts p WHERE p.forum_id = f.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at DESC, p.id DESC LIMIT 1),
            last_post_at      = (SELECT p.created_at FROM forum.posts p WHERE p.forum_id = f.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at DESC, p.id DESC LIMIT 1),
            last_post_user_id = (SELECT p.author_id FROM forum.posts p WHERE p.forum_id = f.id AND p.is_approved AND NOT p.is_deleted ORDER BY p.created_at DESC, p.id DESC LIMIT 1)
         WHERE f.id = $1",
    )
    .bind(forum_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
