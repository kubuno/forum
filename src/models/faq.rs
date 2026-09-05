use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// One question/answer pair of the editable FAQ (phpBB-style): admin-curated,
/// shown to every member on a dedicated page. `answer_md` is Markdown,
/// rendered client-side with the same sanitized renderer as post bodies
/// (`PostBody`) — never on the server, so this table carries the raw text.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FaqEntry {
    pub id:         Uuid,
    pub question:   String,
    pub answer_md:  String,
    pub position:   i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateFaqDto {
    #[validate(length(min = 1, max = 300))]
    pub question: String,
    #[validate(length(min = 1, max = 20000))]
    pub answer_md: String,
    #[serde(default)]
    pub position: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateFaqDto {
    #[validate(length(min = 1, max = 300))]
    pub question: Option<String>,
    #[validate(length(min = 1, max = 20000))]
    pub answer_md: Option<String>,
    pub position: Option<i32>,
}
