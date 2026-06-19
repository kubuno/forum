use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tag {
    pub id:         Uuid,
    pub name:       String,
    pub slug:       String,
    pub color:      String,
    #[sqlx(default)]
    pub topic_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagDto {
    #[validate(length(min = 1, max = 60))]
    pub name:  String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetTopicTagsDto {
    pub tag_ids: Vec<Uuid>,
}
