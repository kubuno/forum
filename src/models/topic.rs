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
    /// False while the opening message waits in the approval queue. The topic
    /// is then invisible to everyone but its author and the moderators.
    pub is_approved:       bool,
    pub approved_at:       Option<DateTime<Utc>>,
    pub approved_by:       Option<Uuid>,
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
    /// Soft-deleted by its author or a moderator: hidden from every listing,
    /// recoverable, visible only to its author and moderators (SEC-12).
    #[serde(default)]
    pub is_deleted:        bool,
    /// When/by whom it was soft-deleted — surfaced to moderators in the trash.
    pub deleted_at:        Option<DateTime<Utc>>,
    pub deleted_by:        Option<Uuid>,
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
    #[validate(length(max = 10))]
    pub tag_ids:    Vec<Uuid>,
    /// Optional poll attached to the opening topic.
    #[validate(nested)]
    pub poll:       Option<CreatePollDto>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePollDto {
    #[validate(length(min = 1, max = 500))]
    pub question:    String,
    #[serde(default)]
    pub is_multiple: bool,
    pub closes_at:   Option<DateTime<Utc>>,
    #[validate(custom(function = "validate_poll_options"))]
    pub options:     Vec<String>,
}

/// A poll needs between 2 and 20 non-empty options, each at most 200 characters.
/// Without this a single request could insert thousands of options, or an
/// over-long option would surface a raw SQL error instead of a clean 400 (SEC-20).
fn validate_poll_options(options: &[String]) -> std::result::Result<(), validator::ValidationError> {
    if !(2..=20).contains(&options.len()) {
        return Err(validator::ValidationError::new("poll_options_count"));
    }
    for option in options {
        let trimmed = option.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 200 {
            return Err(validator::ValidationError::new("poll_option_length"));
        }
    }
    Ok(())
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
    #[validate(length(min = 1, max = 200))]
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
