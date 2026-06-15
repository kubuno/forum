use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::Result;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Subscription {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub topic_id:   Option<Uuid>,
    pub forum_id:   Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReadState {
    pub topic_id:          Uuid,
    pub last_read_post_id: Option<Uuid>,
}

pub struct EngagementService;

impl EngagementService {
    // ── Subscriptions (watch) ─────────────────────────────────────────────────

    pub async fn subscribe_topic(user_id: Uuid, topic_id: Uuid, db: &PgPool) -> Result<Subscription> {
        let row = sqlx::query_as::<_, Subscription>(
            "INSERT INTO forum.subscriptions (user_id, topic_id) VALUES ($1, $2)
             ON CONFLICT (user_id, topic_id) WHERE topic_id IS NOT NULL
             DO UPDATE SET user_id = EXCLUDED.user_id
             RETURNING *",
        )
        .bind(user_id)
        .bind(topic_id)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn subscribe_forum(user_id: Uuid, forum_id: Uuid, db: &PgPool) -> Result<Subscription> {
        let row = sqlx::query_as::<_, Subscription>(
            "INSERT INTO forum.subscriptions (user_id, forum_id) VALUES ($1, $2)
             ON CONFLICT (user_id, forum_id) WHERE forum_id IS NOT NULL
             DO UPDATE SET user_id = EXCLUDED.user_id
             RETURNING *",
        )
        .bind(user_id)
        .bind(forum_id)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn unsubscribe_topic(user_id: Uuid, topic_id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.subscriptions WHERE user_id = $1 AND topic_id = $2")
            .bind(user_id)
            .bind(topic_id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn unsubscribe_forum(user_id: Uuid, forum_id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.subscriptions WHERE user_id = $1 AND forum_id = $2")
            .bind(user_id)
            .bind(forum_id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn list_subscriptions(user_id: Uuid, db: &PgPool) -> Result<Vec<Subscription>> {
        let rows = sqlx::query_as::<_, Subscription>(
            "SELECT * FROM forum.subscriptions WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    // ── Read markers (unread tracking) ────────────────────────────────────────

    pub async fn mark_read(user_id: Uuid, topic_id: Uuid, last_read_post_id: Option<Uuid>, db: &PgPool) -> Result<()> {
        sqlx::query(
            "INSERT INTO forum.read_markers (user_id, topic_id, last_read_post_id, read_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (user_id, topic_id)
             DO UPDATE SET last_read_post_id = EXCLUDED.last_read_post_id, read_at = NOW()",
        )
        .bind(user_id)
        .bind(topic_id)
        .bind(last_read_post_id)
        .execute(db)
        .await?;
        Ok(())
    }

    /// Read markers for all the topics of a forum, for the given user.
    pub async fn read_state_for_forum(user_id: Uuid, forum_id: Uuid, db: &PgPool) -> Result<Vec<ReadState>> {
        let rows = sqlx::query_as::<_, ReadState>(
            "SELECT rm.topic_id, rm.last_read_post_id
               FROM forum.read_markers rm
               JOIN forum.topics t ON t.id = rm.topic_id
              WHERE rm.user_id = $1 AND t.forum_id = $2",
        )
        .bind(user_id)
        .bind(forum_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
