use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Forum {
    pub id:                Uuid,
    pub category_id:       Uuid,
    pub parent_forum_id:   Option<Uuid>,
    pub name:              String,
    pub description:       Option<String>,
    pub position:          i32,
    pub is_locked:         bool,
    pub topic_count:       i32,
    pub post_count:        i32,
    pub last_post_id:      Option<Uuid>,
    pub last_post_at:      Option<DateTime<Utc>>,
    pub last_post_user_id: Option<Uuid>,
    pub color:             Option<String>,
    pub icon:              Option<String>,
    pub is_readonly:       bool,
    pub rules_md:          Option<String>,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateForumDto {
    pub category_id:     Uuid,
    pub parent_forum_id: Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub name:            String,
    #[validate(length(max = 5000))]
    pub description:     Option<String>,
    #[serde(default)]
    pub position:        i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateForumDto {
    pub category_id:     Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub name:            Option<String>,
    #[validate(length(max = 5000))]
    pub description:     Option<String>,
    pub position:        Option<i32>,
    pub is_locked:       Option<bool>,
    pub color:           Option<String>,
    pub icon:            Option<String>,
    pub is_readonly:     Option<bool>,
    #[validate(length(max = 20000))]
    pub rules_md:        Option<String>,
}
