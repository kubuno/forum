use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::Result, models::notification::Notification, services::directory, state::AppState,
};

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

        // Fold high-volume, topic-scoped kinds into the recipient's existing
        // unread notification for the same topic — bumping it to the top and
        // counting one more responder — instead of stacking a new row. Exactly
        // one row is touched (the most recent open one).
        if matches!(kind, "reply" | "reaction") {
            let folded = sqlx::query_scalar::<_, Uuid>(
                "UPDATE forum.notifications
                    SET responder_count = responder_count + 1, actor_id = $4,
                        post_id = $5, created_at = NOW()
                  WHERE id = (SELECT id FROM forum.notifications
                               WHERE user_id = $1 AND kind = $2 AND topic_id = $3 AND is_read = FALSE
                               ORDER BY created_at DESC LIMIT 1)
                 RETURNING id",
            )
            .bind(recipient)
            .bind(kind)
            .bind(topic_id)
            .bind(actor)
            .bind(post_id)
            .fetch_optional(&state.db)
            .await;
            if let Ok(Some(id)) = folded {
                Self::push(state, recipient, id, kind, Some(topic_id), actor).await;
                return;
            }
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
            Ok(id) => Self::push(state, recipient, id, kind, Some(topic_id), actor).await,
            Err(e) => tracing::warn!(error = %e, "forum notification insert failed"),
        }
    }

    /// Like [`notify`], for events not tied to a topic — a warning, a ban, or a
    /// rejected contribution whose topic no longer exists. Stored with a null
    /// `topic_id`, so it cannot be cascade-deleted with a topic and the bell
    /// renders it as a plain entry with nothing to open.
    pub async fn notify_simple(state: &AppState, recipient: Uuid, kind: &str, actor: Uuid, extra: Option<&str>) {
        if recipient == actor {
            return;
        }
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO forum.notifications (user_id, kind, actor_id, topic_id, post_id, extra)
             VALUES ($1, $2, $3, NULL, NULL, $4) RETURNING id",
        )
        .bind(recipient)
        .bind(kind)
        .bind(actor)
        .bind(extra)
        .fetch_one(&state.db)
        .await;
        match inserted {
            Ok(id) => Self::push(state, recipient, id, kind, None, actor).await,
            Err(e) => tracing::warn!(error = %e, "forum notification insert failed"),
        }
    }

    /// Targeted WebSocket delivery through the core (`Custom` event carrying
    /// `recipient_user_ids`). `topic_id` is null for topic-less notifications.
    ///
    /// The title is resolved and the event sent from a spawned task, so a slow
    /// (or unavailable) directory lookup never delays the caller: `notify`
    /// returns as soon as the row is written. Naming the actor requires a
    /// round trip to the core's directory (best-effort, short-timeout,
    /// short-TTL cached — see [`directory::display_name`]); on any failure we
    /// fall back to the generic, kind-based title so a native (mobile) push is
    /// still never blank.
    async fn push(state: &AppState, recipient: Uuid, id: Uuid, kind: &str, topic_id: Option<Uuid>, actor: Uuid) {
        let url = format!("{}/internal/events/publish", state.settings.core.url);
        let secret = state.settings.core.internal_secret.clone();
        let owned_state = state.clone();
        let kind = kind.to_string();
        tokio::spawn(async move {
            let (fallback_title, body) = Self::push_text(&kind);
            let title = match directory::display_name(&owned_state, actor).await {
                Some(name) => Self::rich_title(&kind, &name),
                None => fallback_title.to_string(),
            };
            let event = json!({
                "type": "Custom",
                "payload": {
                    "event_type": "forum.notification",
                    "module_id":  "forum",
                    "payload": {
                        "recipient_user_ids": [recipient.to_string()],
                        "notification_id":    id.to_string(),
                        "kind":               kind,
                        "topic_id":           topic_id.map(|t| t.to_string()),
                        "title":              title,
                        "body":               body,
                        // Standard cross-module field: where the shared header bell
                        // sends the reader when they click the notification.
                        "link":               topic_id.map(|t| format!("/forum/topics/{t}")),
                    }
                }
            });
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

    /// A rich, actor-named title (e.g. "Alice a répondu"), used when the
    /// core's directory resolved the actor's display name. Falls back to the
    /// generic [`push_text`] title for kinds it doesn't recognize.
    fn rich_title(kind: &str, actor_name: &str) -> String {
        let verb = match kind {
            "reply"           => "a répondu",
            "mention"         => "vous a mentionné",
            "reaction"        => "a réagi",
            "solution"        => "a marqué votre message comme solution",
            "quote"           => "a cité votre message",
            "topic"           => "a publié un nouveau sujet",
            "report"          => "a signalé un message",
            "report_resolved" => "a traité un signalement",
            "approved"        => "a approuvé votre message",
            "rejected"        => "a rejeté votre message",
            "warning"         => "vous a averti",
            "ban"             => "vous a banni",
            _                 => return Self::push_text(kind).0.to_string(),
        };
        format!("{actor_name} {verb}")
    }

    /// Generic lock-screen title/body per notification kind (French, the board's
    /// default), used as a fallback when the actor's name could not be resolved.
    fn push_text(kind: &str) -> (&'static str, &'static str) {
        match kind {
            "reply"           => ("Nouvelle réponse", "Quelqu'un a répondu à un sujet que vous suivez"),
            "mention"         => ("Vous avez été mentionné", "Quelqu'un vous a mentionné dans un message"),
            "reaction"        => ("Nouvelle réaction", "Quelqu'un a réagi à votre message"),
            "solution"        => ("Solution acceptée", "Votre message a été marqué comme solution"),
            "quote"           => ("Citation", "Quelqu'un a cité votre message"),
            "topic"           => ("Nouveau sujet", "Un nouveau sujet a été publié dans un forum que vous suivez"),
            "report"          => ("Nouveau signalement", "Un message a été signalé à la modération"),
            "report_resolved" => ("Signalement traité", "Votre signalement a été traité"),
            "approved"        => ("Message approuvé", "Votre message a été publié"),
            "rejected"        => ("Message rejeté", "Votre message n'a pas été retenu"),
            "warning"         => ("Avertissement", "Vous avez reçu un avertissement"),
            "ban"             => ("Bannissement", "Vous avez été banni du forum"),
            _                 => ("Forum", "Vous avez une nouvelle notification"),
        }
    }

    /// Notifies a private-message thread participant of a new message (or of
    /// having just been added to a new thread), through the same shared bell
    /// as the board notifications. Deliberately narrower than [`notify`]: a
    /// PM thread has no `topic_id` (it isn't a `forum.topics` row, so it
    /// cannot go through the FK-checked `topic_id` column — see migration
    /// `000013`), and the push body must NEVER carry the message text, only
    /// the fact that one arrived. `thread_id` is kept in `extra` purely for
    /// operators reading the raw table; nothing renders it back as content.
    pub async fn notify_pm(state: &AppState, recipient: Uuid, actor: Uuid, thread_id: Uuid) {
        if recipient == actor {
            return;
        }
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO forum.notifications (user_id, kind, actor_id, topic_id, post_id, extra)
             VALUES ($1, 'pm', $2, NULL, NULL, $3) RETURNING id",
        )
        .bind(recipient)
        .bind(actor)
        .bind(thread_id.to_string())
        .fetch_one(&state.db)
        .await;
        match inserted {
            Ok(id) => Self::push_pm(state, recipient, id, actor, thread_id).await,
            Err(e) => tracing::warn!(error = %e, "forum PM notification insert failed"),
        }
    }

    /// Like [`push`], but for private messages: the link points at the PM
    /// thread instead of a topic, and the body is a fixed, content-free
    /// string — never the message the sender wrote.
    async fn push_pm(state: &AppState, recipient: Uuid, id: Uuid, actor: Uuid, thread_id: Uuid) {
        let url = format!("{}/internal/events/publish", state.settings.core.url);
        let secret = state.settings.core.internal_secret.clone();
        let owned_state = state.clone();
        tokio::spawn(async move {
            let title = directory::display_name(&owned_state, actor)
                .await
                .unwrap_or_else(|| "Forum".to_string());
            let event = json!({
                "type": "Custom",
                "payload": {
                    "event_type": "forum.notification",
                    "module_id":  "forum",
                    "payload": {
                        "recipient_user_ids": [recipient.to_string()],
                        "notification_id":    id.to_string(),
                        "kind":               "pm",
                        "topic_id":            serde_json::Value::Null,
                        "title":               title,
                        "body":                "Nouveau message privé",
                        // Standard cross-module field: where the shared header bell
                        // sends the reader when they click the notification.
                        "link":                format!("/forum/pm/{thread_id}"),
                    }
                }
            });
            if let Err(e) = reqwest::Client::new()
                .post(&url)
                .header("X-Internal-Secret", secret)
                .json(&event)
                .send()
                .await
            {
                tracing::warn!(error = %e, "forum PM notification push failed");
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
