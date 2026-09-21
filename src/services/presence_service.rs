use chrono::{Duration, Utc};
use kubuno_db::dialect::Assign;
use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::errors::Result;

pub struct PresenceService;

impl PresenceService {
    /// Records that a user is active. Called by a lightweight client heartbeat.
    pub async fn heartbeat(user_id: Uuid, path: Option<&str>, db: &DbPool) -> Result<()> {
        let now = Utc::now();
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.online (user_id, last_seen_at, path) VALUES ($1, $2, $3){}",
            b.upsert(
                "online",
                &["user_id"],
                &[Assign::Incoming("last_seen_at"), Assign::Incoming("path")],
            )
        );
        db.execute(&sql, params![user_id, now, path]).await?;
        db.execute(
            "UPDATE forum.user_profiles SET last_seen_at = $1 WHERE user_id = $2",
            params![now, user_id],
        )
        .await?;
        Ok(())
    }

    /// User ids seen within the last `within_min` minutes.
    pub async fn who_online(within_min: i64, db: &DbPool) -> Result<Vec<Uuid>> {
        let cutoff = Utc::now() - Duration::minutes(within_min.clamp(1, 120));
        let ids: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT user_id FROM forum.online \
                 WHERE last_seen_at > $1 \
                 ORDER BY last_seen_at DESC LIMIT 200",
                params![cutoff],
            )
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();
        Ok(ids)
    }

    /// Same as `who_online`, but also returns each user's last-heartbeat path
    /// (the raw client path, e.g. `/forum/topics/:id`) for the "who's online"
    /// detail page. The caller is responsible for turning that path into a
    /// location the requester is actually allowed to see.
    pub async fn who_online_with_path(within_min: i64, db: &DbPool) -> Result<Vec<(Uuid, Option<String>)>> {
        let cutoff = Utc::now() - Duration::minutes(within_min.clamp(1, 120));
        let rows = db
            .fetch_all_as::<(Uuid, Option<String>)>(
                "SELECT user_id, path FROM forum.online \
                 WHERE last_seen_at > $1 \
                 ORDER BY last_seen_at DESC LIMIT 200",
                params![cutoff],
            )
            .await?;
        Ok(rows)
    }
}
