use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id:               Uuid,
    pub topic_id:         Uuid,
    pub forum_id:         Uuid,
    pub author_id:        Uuid,
    pub body_md:          String,
    pub reply_to_post_id: Option<Uuid>,
    pub is_first_post:    bool,
    /// False while the message waits in the approval queue (see
    /// `forum.post_approval_mode`). Only its author and moderators see it.
    pub is_approved:      bool,
    pub approved_at:      Option<DateTime<Utc>>,
    pub approved_by:      Option<Uuid>,
    /// Soft-deleted by a moderator: visible only to its author and moderators.
    #[serde(default)]
    pub is_deleted:       bool,
    pub edited_at:        Option<DateTime<Utc>>,
    pub edited_by:        Option<Uuid>,
    pub edit_reason:      Option<String>,
    pub edit_count:       i32,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
}

/// A past version of a post's body, archived just before an edit overwrites
/// it. Visible only to the post's author and to moderators (see
/// `handlers::posts::list_revisions`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostRevision {
    pub id:          Uuid,
    pub post_id:     Uuid,
    pub body_md:     String,
    pub edited_by:   Option<Uuid>,
    pub edit_reason: Option<String>,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostDto {
    #[validate(length(min = 1, max = 100000))]
    pub body_md:          String,
    pub reply_to_post_id: Option<Uuid>,
    /// User ids picked by the @mention autocomplete on the client. Capped to keep
    /// a single request from fanning out into thousands of notification inserts.
    #[serde(default)]
    #[validate(length(max = 20))]
    pub mention_user_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePostDto {
    #[validate(length(min = 1, max = 100000))]
    pub body_md:     String,
    #[validate(length(max = 500))]
    pub edit_reason: Option<String>,
}
