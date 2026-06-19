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
    pub created_at: DateTime<Utc>,
}
