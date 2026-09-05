use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub kind:       String,
    pub actor_id:   Option<Uuid>,
    pub topic_id:   Option<Uuid>,
    pub post_id:    Option<Uuid>,
    pub extra:      Option<String>,
    pub is_read:    bool,
    /// How many actors this notification stands for (1, or more when replies to
    /// the same topic have been folded together).
    #[serde(default)]
    pub responder_count: i32,
    pub created_at: DateTime<Utc>,
}
