//! Denormalised counter maintenance. Recomputing from the source rows after each
//! mutation keeps the cached aggregates correct without fragile incremental logic.
//!
//! The recompute reads the source rows and writes the result back from Rust
//! rather than in one correlated `UPDATE ... SET x = (SELECT ...)`: PostgreSQL's
//! `GREATEST` has no SQLite equivalent, and a self-correlated update subquery is
//! a portability minefield, so the arithmetic lives here where all three engines
//! behave identically.

use chrono::{DateTime, Utc};
use kubuno_db::{params, DbTx};
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

/// The newest approved, non-deleted post of a scope, as (id, created_at, author).
async fn last_post(
    tx: &mut DbTx,
    scope_col: &'static str,
    scope_id: Uuid,
) -> Result<(Option<Uuid>, Option<DateTime<Utc>>, Option<Uuid>)> {
    let sql = format!(
        "SELECT id, created_at, author_id FROM forum.posts \
          WHERE {scope_col} = $1 AND is_approved AND NOT is_deleted \
          ORDER BY created_at DESC, id DESC LIMIT 1"
    );
    match tx.fetch_optional_row(&sql, params![scope_id]).await? {
        Some(r) => Ok((
            Some(r.try_get::<Uuid>("id")?),
            Some(r.try_get::<DateTime<Utc>>("created_at")?),
            Some(r.try_get::<Uuid>("author_id")?),
        )),
        None => Ok((None, None, None)),
    }
}

/// Recompute a topic's first/last post pointers and reply count.
///
/// Only APPROVED messages count. These aggregates are what the lists show —
/// "3 replies, last one an hour ago" — so counting a message still waiting in
/// the moderation queue would advertise content the reader cannot open, and
/// would leak that someone posted something a moderator has not released.
pub async fn recompute_topic(tx: &mut DbTx, topic_id: Uuid) -> Result<()> {
    let count: i64 = tx
        .fetch_optional_scalar(
            "SELECT COUNT(*) FROM forum.posts \
              WHERE topic_id = $1 AND is_approved AND NOT is_deleted",
            params![topic_id],
        )
        .await?
        .unwrap_or(0);
    // reply_count is an INT column: bind the value at that width.
    let reply_count = (count - 1).max(0) as i32;

    let first_post_id: Option<Uuid> = tx
        .fetch_optional_scalar(
            "SELECT id FROM forum.posts \
              WHERE topic_id = $1 AND is_approved AND NOT is_deleted \
              ORDER BY created_at, id LIMIT 1",
            params![topic_id],
        )
        .await?;

    let (last_id, last_at, last_user) = last_post(tx, "topic_id", topic_id).await?;

    tx.execute(
        "UPDATE forum.topics SET reply_count = $1, first_post_id = $2, \
                last_post_id = $3, last_post_at = $4, last_post_user_id = $5 \
          WHERE id = $6",
        params![
            reply_count,
            first_post_id,
            last_id,
            last_at,
            last_user,
            topic_id
        ],
    )
    .await?;
    Ok(())
}

/// Recompute a forum's topic/post counts and last-post pointers. Approved rows
/// only, for the same reason as `recompute_topic`.
pub async fn recompute_forum(tx: &mut DbTx, forum_id: Uuid) -> Result<()> {
    // topic_count / post_count are INT columns: bind the values at that width.
    let topic_count = tx
        .fetch_optional_scalar::<i64>(
            "SELECT COUNT(*) FROM forum.topics \
              WHERE forum_id = $1 AND is_approved AND NOT is_deleted",
            params![forum_id],
        )
        .await?
        .unwrap_or(0) as i32;
    let post_count = tx
        .fetch_optional_scalar::<i64>(
            "SELECT COUNT(*) FROM forum.posts \
              WHERE forum_id = $1 AND is_approved AND NOT is_deleted",
            params![forum_id],
        )
        .await?
        .unwrap_or(0) as i32;

    let (last_id, last_at, last_user) = last_post(tx, "forum_id", forum_id).await?;

    tx.execute(
        "UPDATE forum.forums SET topic_count = $1, post_count = $2, \
                last_post_id = $3, last_post_at = $4, last_post_user_id = $5 \
          WHERE id = $6",
        params![
            topic_count,
            post_count,
            last_id,
            last_at,
            last_user,
            forum_id
        ],
    )
    .await?;
    Ok(())
}
