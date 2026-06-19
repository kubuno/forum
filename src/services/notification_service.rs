use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::Result, models::notification::Notification, state::AppState};

pub struct NotificationService;

impl NotificationService {
    /// Inserts a notification and best-effort pushes it to the recipient over the
    /// core WebSocket. Never fails the caller (logs on error). No-op when the
    /// recipient is the actor.
    pub async fn notify(
        state: &AppState,
        recipient: Uuid,
        kind: &str,
        actor: Uuid,
        topic_id: Uuid,
        post_id: Option<Uuid>,
        extra: Option<&str>,
    ) {
        if recipient == actor {
            return;
        }
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO forum.notifications (user_id, kind, actor_id, topic_id, post_id, extra)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(recipient)
        .bind(kind)
        .bind(actor)
        .bind(topic_id)
        .bind(post_id)
        .bind(extra)
        .fetch_one(&state.db)
        .await;

        match inserted {
            Ok(id) => Self::push(state, recipient, id, kind, topic_id).await,
            Err(e) => tracing::warn!(error = %e, "forum notification insert failed"),
        }
    }

    /// Targeted WebSocket delivery through the core (`Custom` event carrying
    /// `recipient_user_ids`).
    async fn push(state: &AppState, recipient: Uuid, id: Uuid, kind: &str, topic_id: Uuid) {
        let event = json!({
            "type": "Custom",
            "payload": {
                "event_type": "forum.notification",
                "module_id":  "forum",
                "payload": {
                    "recipient_user_ids": [recipient.to_string()],
                    "notification_id":    id.to_string(),
                    "kind":               kind,
                    "topic_id":           topic_id.to_string(),
                }
            }
        });
        let url = format!("{}/internal/events/publish", state.settings.core.url);
        let secret = state.settings.core.internal_secret.clone();
        tokio::spawn(async move {
            if let Err(e) = reqwest::Client::new()
                .post(&url)
                .header("X-Internal-Secret", secret)
                .json(&event)
                .send()
                .await
            {
                tracing::warn!(error = %e, "forum notification push failed");
            }
        });
    }

    pub async fn list(user_id: Uuid, only_unread: bool, limit: i64, db: &PgPool) -> Result<Vec<Notification>> {
        let rows = sqlx::query_as::<_, Notification>(
            "SELECT * FROM forum.notifications
             WHERE user_id = $1 AND ($2 = FALSE OR is_read = FALSE)
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(user_id)
        .bind(only_unread)
        .bind(limit.clamp(1, 200))
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn unread_count(user_id: Uuid, db: &PgPool) -> Result<i64> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM forum.notifications WHERE user_id = $1 AND is_read = FALSE",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(n)
    }

    pub async fn mark_read(user_id: Uuid, id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("UPDATE forum.notifications SET is_read = TRUE WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn mark_all_read(user_id: Uuid, db: &PgPool) -> Result<u64> {
        let r = sqlx::query("UPDATE forum.notifications SET is_read = TRUE WHERE user_id = $1 AND is_read = FALSE")
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(r.rows_affected())
    }
}
