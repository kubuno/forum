//! Ignore list (phpBB-style "foes"/zebra): see migration `000015`.
//!
//! SECURITY: purely a display preference for the caller's own client. A
//! member manages ONLY their own list — every method here is keyed off the
//! `user_id` extracted from the authenticated session, never a path/body
//! parameter standing in for "which user's list". Ignoring someone never
//! changes what the backend returns from post/topic listing endpoints — the
//! frontend is the one folding an ignored author's messages, and the user
//! can always reveal them again.

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::{ForumError, Result};

pub struct IgnoreService;

impl IgnoreService {
    /// `GET /me/ignored` — ids the caller has ignored, most recent first.
    pub async fn list(user_id: Uuid, db: &PgPool) -> Result<Vec<Uuid>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT ignored_user_id FROM forum.ignored_users
              WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// `POST /me/ignored` — idempotent. Refuses ignoring oneself.
    pub async fn add(user_id: Uuid, target: Uuid, db: &PgPool) -> Result<()> {
        if target == user_id {
            return Err(ForumError::Validation("impossible de s'ignorer soi-même".into()));
        }
        sqlx::query(
            "INSERT INTO forum.ignored_users (user_id, ignored_user_id) VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(target)
        .execute(db)
        .await?;
        Ok(())
    }

    /// `DELETE /me/ignored/:uid`.
    pub async fn remove(user_id: Uuid, target: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.ignored_users WHERE user_id = $1 AND ignored_user_id = $2")
            .bind(user_id)
            .bind(target)
            .execute(db)
            .await?;
        Ok(())
    }
}
