//! Ignore list: see migration `000015`.
//!
//! SECURITY: purely a display preference for the caller's own client. A
//! member manages ONLY their own list — every method here is keyed off the
//! `user_id` extracted from the authenticated session, never a path/body
//! parameter standing in for "which user's list". Ignoring someone never
//! changes what the backend returns from post/topic listing endpoints — the
//! frontend is the one folding an ignored author's messages, and the user
//! can always reveal them again.

use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::errors::{ForumError, Result};

pub struct IgnoreService;

impl IgnoreService {
    /// `GET /me/ignored` — ids the caller has ignored, most recent first.
    pub async fn list(user_id: Uuid, db: &DbPool) -> Result<Vec<Uuid>> {
        let rows: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT ignored_user_id FROM forum.ignored_users \
                  WHERE user_id = $1 ORDER BY created_at DESC",
                params![user_id],
            )
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();
        Ok(rows)
    }

    /// `POST /me/ignored` — idempotent. Refuses ignoring oneself.
    pub async fn add(user_id: Uuid, target: Uuid, db: &DbPool) -> Result<()> {
        if target == user_id {
            return Err(ForumError::Validation("impossible de s'ignorer soi-même".into()));
        }
        let b = db.backend();
        let sql = format!(
            "INSERT {}INTO forum.ignored_users (user_id, ignored_user_id) VALUES ($1, $2){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(&["user_id", "ignored_user_id"]),
        );
        db.execute(&sql, params![user_id, target]).await?;
        Ok(())
    }

    /// `DELETE /me/ignored/:uid`.
    pub async fn remove(user_id: Uuid, target: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM forum.ignored_users WHERE user_id = $1 AND ignored_user_id = $2",
            params![user_id, target],
        )
        .await?;
        Ok(())
    }
}
