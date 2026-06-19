use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Draft {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub forum_id:   Option<Uuid>,
    pub topic_id:   Option<Uuid>,
    pub title:      Option<String>,
    pub body_md:    String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SaveDraftDto {
    pub forum_id: Option<Uuid>,
    pub topic_id: Option<Uuid>,
    pub title:    Option<String>,
    pub body_md:  String,
}
