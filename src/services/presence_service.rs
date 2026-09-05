use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::Result;

pub struct PresenceService;

impl PresenceService {
    /// Records that a user is active. Called by a lightweight client heartbeat.
    pub async fn heartbeat(user_id: Uuid, path: Option<&str>, db: &PgPool) -> Result<()> {
        sqlx::query(
            "INSERT INTO forum.online (user_id, last_seen_at, path)
             VALUES ($1, NOW(), $2)
             ON CONFLICT (user_id) DO UPDATE SET last_seen_at = NOW(), path = EXCLUDED.path",
        )
        .bind(user_id)
        .bind(path)
        .execute(db)
        .await?;
        sqlx::query("UPDATE forum.user_profiles SET last_seen_at = NOW() WHERE user_id = $1")
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// User ids seen within the last `within_min` minutes.
    pub async fn who_online(within_min: i64, db: &PgPool) -> Result<Vec<Uuid>> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM forum.online
             WHERE last_seen_at > NOW() - make_interval(mins => $1::int)
             ORDER BY last_seen_at DESC LIMIT 200",
        )
        .bind(within_min.clamp(1, 120) as i32)
        .fetch_all(db)
        .await?;
        Ok(ids)
    }

    /// Same as `who_online`, but also returns each user's last-heartbeat path
    /// (the raw client path, e.g. `/forum/topics/:id`) for the "who's online"
    /// detail page. The caller is responsible for turning that path into a
    /// location the requester is actually allowed to see.
    pub async fn who_online_with_path(within_min: i64, db: &PgPool) -> Result<Vec<(Uuid, Option<String>)>> {
        let rows = sqlx::query_as::<_, (Uuid, Option<String>)>(
            "SELECT user_id, path FROM forum.online
             WHERE last_seen_at > NOW() - make_interval(mins => $1::int)
             ORDER BY last_seen_at DESC LIMIT 200",
        )
        .bind(within_min.clamp(1, 120) as i32)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
