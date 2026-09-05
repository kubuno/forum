use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{errors::Result, middleware::ForumUser, services::permission_service::PermissionService};

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

    /// Everyone watching a topic — directly, or through a subscription to its
    /// forum — minus `exclude` (the author of the new message). Callers still
    /// apply a visibility gate before notifying, so this only gathers candidates.
    pub async fn topic_watchers(topic_id: Uuid, forum_id: Uuid, exclude: Uuid, db: &PgPool) -> Result<Vec<Uuid>> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT user_id FROM forum.subscriptions
              WHERE (topic_id = $1 OR forum_id = $2) AND user_id <> $3",
        )
        .bind(topic_id)
        .bind(forum_id)
        .bind(exclude)
        .fetch_all(db)
        .await?;
        Ok(ids)
    }

    /// Everyone watching a forum, minus `exclude` — the candidates to notify of a
    /// new topic there.
    pub async fn forum_watchers(forum_id: Uuid, exclude: Uuid, db: &PgPool) -> Result<Vec<Uuid>> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT user_id FROM forum.subscriptions WHERE forum_id = $1 AND user_id <> $2",
        )
        .bind(forum_id)
        .bind(exclude)
        .fetch_all(db)
        .await?;
        Ok(ids)
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

    /// "Mark forum read": upserts a read marker for every topic of the forum
    /// that is visible to the caller (not soft-deleted, and either approved or
    /// authored by them), regardless of when the topic last received a post.
    /// The caller must already have checked `can_view` on the forum.
    pub async fn mark_forum_read(user_id: Uuid, forum_id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query(
            "INSERT INTO forum.read_markers (user_id, topic_id, read_at)
             SELECT $1, t.id, NOW()
               FROM forum.topics t
              WHERE t.forum_id = $2 AND t.is_deleted = FALSE
                AND (t.is_approved = TRUE OR t.author_id = $1)
             ON CONFLICT (user_id, topic_id) DO UPDATE SET read_at = NOW()",
        )
        .bind(user_id)
        .bind(forum_id)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, forum_id = %forum_id, "failed to mark forum read");
            e
        })?;
        Ok(())
    }

    /// "Mark all read": same as `mark_forum_read` but across every forum the
    /// caller can currently see, reusing `PermissionService::push_visible_forum`
    /// so this can never diverge from the visibility rules used elsewhere.
    pub async fn mark_all_read(user: &ForumUser, db: &PgPool) -> Result<()> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO forum.read_markers (user_id, topic_id, read_at) SELECT ",
        );
        qb.push_bind(user.id);
        qb.push(", t.id, NOW() FROM forum.topics t JOIN forum.forums f ON f.id = t.forum_id \
                  WHERE t.is_deleted = FALSE AND (t.is_approved = TRUE OR t.author_id = ");
        qb.push_bind(user.id);
        qb.push(") AND ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        qb.push(" ON CONFLICT (user_id, topic_id) DO UPDATE SET read_at = NOW()");

        qb.build().execute(db).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user.id, "failed to mark all read");
            e
        })?;
        Ok(())
    }
}
