use chrono::{DateTime, Utc};
use kubuno_db::dialect::Assign;
use kubuno_db::{new_id, params, DbPool, DbQueryBuilder};
use serde::Serialize;
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

    pub async fn subscribe_topic(user_id: Uuid, topic_id: Uuid, db: &DbPool) -> Result<Subscription> {
        let b = db.backend();
        // The former partial-index conflict target is now a plain UNIQUE
        // (user_id, topic_id); an existing subscription is a no-op.
        let sql = format!(
            "INSERT {}INTO forum.subscriptions (id, user_id, topic_id) VALUES ($1, $2, $3){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(&["user_id", "topic_id"])
        );
        db.execute(&sql, params![new_id(), user_id, topic_id]).await?;
        db.fetch_one_as::<Subscription>(
            "SELECT * FROM forum.subscriptions WHERE user_id = $1 AND topic_id = $2",
            params![user_id, topic_id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn subscribe_forum(user_id: Uuid, forum_id: Uuid, db: &DbPool) -> Result<Subscription> {
        let b = db.backend();
        let sql = format!(
            "INSERT {}INTO forum.subscriptions (id, user_id, forum_id) VALUES ($1, $2, $3){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(&["user_id", "forum_id"])
        );
        db.execute(&sql, params![new_id(), user_id, forum_id]).await?;
        db.fetch_one_as::<Subscription>(
            "SELECT * FROM forum.subscriptions WHERE user_id = $1 AND forum_id = $2",
            params![user_id, forum_id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn unsubscribe_topic(user_id: Uuid, topic_id: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM forum.subscriptions WHERE user_id = $1 AND topic_id = $2",
            params![user_id, topic_id],
        )
        .await?;
        Ok(())
    }

    pub async fn unsubscribe_forum(user_id: Uuid, forum_id: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM forum.subscriptions WHERE user_id = $1 AND forum_id = $2",
            params![user_id, forum_id],
        )
        .await?;
        Ok(())
    }

    pub async fn list_subscriptions(user_id: Uuid, db: &DbPool) -> Result<Vec<Subscription>> {
        let rows = db
            .fetch_all_as::<Subscription>(
                "SELECT * FROM forum.subscriptions WHERE user_id = $1 ORDER BY created_at DESC",
                params![user_id],
            )
            .await?;
        Ok(rows)
    }

    /// Everyone watching a topic — directly, or through a subscription to its
    /// forum — minus `exclude` (the author of the new message).
    pub async fn topic_watchers(topic_id: Uuid, forum_id: Uuid, exclude: Uuid, db: &DbPool) -> Result<Vec<Uuid>> {
        let ids: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT DISTINCT user_id FROM forum.subscriptions \
                  WHERE (topic_id = $1 OR forum_id = $2) AND user_id <> $3",
                params![topic_id, forum_id, exclude],
            )
            .await?
            .into_iter()
            .map(|(u,)| u)
            .collect();
        Ok(ids)
    }

    /// Everyone watching a forum, minus `exclude`.
    pub async fn forum_watchers(forum_id: Uuid, exclude: Uuid, db: &DbPool) -> Result<Vec<Uuid>> {
        let ids: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT DISTINCT user_id FROM forum.subscriptions WHERE forum_id = $1 AND user_id <> $2",
                params![forum_id, exclude],
            )
            .await?
            .into_iter()
            .map(|(u,)| u)
            .collect();
        Ok(ids)
    }

    // ── Read markers (unread tracking) ────────────────────────────────────────

    pub async fn mark_read(user_id: Uuid, topic_id: Uuid, last_read_post_id: Option<Uuid>, db: &DbPool) -> Result<()> {
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.read_markers (user_id, topic_id, last_read_post_id, read_at) \
             VALUES ($1, $2, $3, $4){}",
            b.upsert(
                "read_markers",
                &["user_id", "topic_id"],
                &[Assign::Incoming("last_read_post_id"), Assign::Incoming("read_at")],
            )
        );
        db.execute(&sql, params![user_id, topic_id, last_read_post_id, Utc::now()]).await?;
        Ok(())
    }

    /// Sets (or refreshes) a read marker's timestamp without touching
    /// `last_read_post_id` — the "mark read" bulk operations' per-topic write.
    async fn bump_read(db: &DbPool, user_id: Uuid, topic_id: Uuid) -> Result<()> {
        let b = db.backend();
        let sql = format!(
            "INSERT INTO forum.read_markers (user_id, topic_id, read_at) VALUES ($1, $2, $3){}",
            b.upsert("read_markers", &["user_id", "topic_id"], &[Assign::Incoming("read_at")])
        );
        db.execute(&sql, params![user_id, topic_id, Utc::now()]).await?;
        Ok(())
    }

    /// Read markers for all the topics of a forum, for the given user.
    pub async fn read_state_for_forum(user_id: Uuid, forum_id: Uuid, db: &DbPool) -> Result<Vec<ReadState>> {
        let rows = db
            .fetch_all_as::<ReadState>(
                "SELECT rm.topic_id, rm.last_read_post_id \
                   FROM forum.read_markers rm \
                   JOIN forum.topics t ON t.id = rm.topic_id \
                  WHERE rm.user_id = $1 AND t.forum_id = $2",
                params![user_id, forum_id],
            )
            .await?;
        Ok(rows)
    }

    /// "Mark forum read": marks every topic of the forum visible to the caller
    /// (not soft-deleted, and either approved or authored by them). The caller
    /// must already have checked `can_view` on the forum.
    ///
    /// The candidate topics are gathered first and each marker is upserted
    /// individually: an `INSERT ... SELECT ... ON CONFLICT DO UPDATE` has no
    /// portable form (MySQL cannot read the inserted value in the update branch
    /// of an `INSERT ... SELECT`).
    pub async fn mark_forum_read(user_id: Uuid, forum_id: Uuid, db: &DbPool) -> Result<()> {
        let topics = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT t.id FROM forum.topics t \
                  WHERE t.forum_id = $1 AND t.is_deleted = FALSE \
                    AND (t.is_approved = TRUE OR t.author_id = $2)",
                params![forum_id, user_id],
            )
            .await?;
        for (topic_id,) in topics {
            Self::bump_read(db, user_id, topic_id).await?;
        }
        Ok(())
    }

    /// "Mark all read": every topic the caller can currently see, reusing
    /// `PermissionService::push_visible_forum` so visibility can never diverge.
    pub async fn mark_all_read(user: &ForumUser, db: &DbPool) -> Result<()> {
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT t.id FROM forum.topics t JOIN forum.forums f ON f.id = t.forum_id \
              WHERE t.is_deleted = FALSE AND (t.is_approved = TRUE OR t.author_id = ",
        );
        qb.push_bind(user.id).push(") AND ");
        PermissionService::push_visible_forum(&mut qb, "f.id", user);
        let topics = qb.fetch_all_as::<(Uuid,)>(db).await?;
        for (topic_id,) in topics {
            Self::bump_read(db, user.id, topic_id).await?;
        }
        Ok(())
    }
}
