use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Topic {
    pub id:                Uuid,
    pub forum_id:          Uuid,
    pub author_id:         Uuid,
    pub title:             String,
    pub slug:              String,
    pub topic_type:        String,
    pub is_locked:         bool,
    pub is_approved:       bool,
    pub view_count:        i32,
    pub reply_count:       i32,
    pub first_post_id:     Option<Uuid>,
    pub last_post_id:      Option<Uuid>,
    pub last_post_at:      Option<DateTime<Utc>>,
    pub last_post_user_id: Option<Uuid>,
    pub is_solved:         bool,
    pub solution_post_id:  Option<Uuid>,
    pub is_question:       bool,
    pub prefix:            Option<String>,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
}

/// Creating a topic also creates its first post (the opening message).
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTopicDto {
    #[validate(length(min = 1, max = 500))]
    pub title:      String,
    #[validate(length(min = 1, max = 100000))]
    pub body_md:    String,
    pub topic_type: Option<String>,
    #[serde(default)]
    pub is_question: bool,
    pub prefix:     Option<String>,
    #[serde(default)]
    pub tag_ids:    Vec<Uuid>,
    /// Optional poll attached to the opening topic.
    pub poll:       Option<CreatePollDto>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePollDto {
    #[validate(length(min = 1, max = 500))]
    pub question:    String,
    #[serde(default)]
    pub is_multiple: bool,
    pub closes_at:   Option<DateTime<Utc>>,
    pub options:     Vec<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTopicDto {
    #[validate(length(min = 1, max = 500))]
    pub title:      Option<String>,
    pub topic_type: Option<String>,
    pub is_locked:  Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MoveTopicDto {
    pub forum_id: Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SplitTopicDto {
    /// Posts to move out into a new topic.
    pub post_ids: Vec<Uuid>,
    #[validate(length(min = 1, max = 500))]
    pub title:    String,
    /// Target forum for the new topic (defaults to the source forum).
    pub forum_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct MergeTopicDto {
    /// Topic whose posts are merged into this one, then removed.
    pub source_topic_id: Uuid,
}
