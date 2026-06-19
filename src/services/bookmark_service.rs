use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::Result, models::topic::Topic};

pub struct BookmarkService;

impl BookmarkService {
    /// Toggles a bookmark on a topic. Returns whether it is now bookmarked.
    pub async fn toggle(user_id: Uuid, topic_id: Uuid, db: &PgPool) -> Result<bool> {
        let existed: Option<(Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM forum.bookmarks WHERE user_id = $1 AND topic_id = $2",
        )
        .bind(user_id)
        .bind(topic_id)
        .fetch_optional(db)
        .await?;
        if existed.is_some() {
            sqlx::query("DELETE FROM forum.bookmarks WHERE user_id = $1 AND topic_id = $2")
                .bind(user_id)
                .bind(topic_id)
                .execute(db)
                .await?;
            Ok(false)
        } else {
            sqlx::query(
                "INSERT INTO forum.bookmarks (user_id, topic_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(user_id)
            .bind(topic_id)
            .execute(db)
            .await?;
            Ok(true)
        }
    }

    pub async fn is_bookmarked(user_id: Uuid, topic_id: Uuid, db: &PgPool) -> Result<bool> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM forum.bookmarks WHERE user_id = $1 AND topic_id = $2",
        )
        .bind(user_id)
        .bind(topic_id)
        .fetch_one(db)
        .await?;
        Ok(n > 0)
    }

    pub async fn list(user_id: Uuid, db: &PgPool) -> Result<Vec<Topic>> {
        let topics = sqlx::query_as::<_, Topic>(
            "SELECT t.* FROM forum.topics t
             JOIN forum.bookmarks b ON b.topic_id = t.id
             WHERE b.user_id = $1
             ORDER BY b.created_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(topics)
    }
}
