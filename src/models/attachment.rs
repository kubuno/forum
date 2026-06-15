use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Attachment {
    pub id:         Uuid,
    pub post_id:    Uuid,
    pub file_id:    Option<Uuid>,
    pub filename:   String,
    pub mime_type:  Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateAttachmentDto {
    /// Reference to a file already uploaded in the drive module.
    pub file_id:    Option<Uuid>,
    #[validate(length(min = 1, max = 500))]
    pub filename:   String,
    pub mime_type:  Option<String>,
    pub size_bytes: Option<i64>,
}
