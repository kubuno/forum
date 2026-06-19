use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Report {
    pub id:          Uuid,
    pub post_id:     Uuid,
    pub reporter_id: Uuid,
    pub reason:      String,
    pub status:      String,
    pub handled_by:  Option<Uuid>,
    pub handled_at:  Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReportDto {
    #[validate(length(min = 1, max = 2000))]
    pub reason: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResolveReportDto {
    /// New status: 'resolved' or 'rejected'.
    #[validate(length(min = 1, max = 20))]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Moderator {
    pub forum_id:   Uuid,
    pub user_id:    Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddModeratorDto {
    pub user_id: Uuid,
}

// ── Advanced moderation: log, warnings, bans, notes ──────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ModLogEntry {
    pub id:             i64,
    pub moderator_id:   Uuid,
    pub action:         String,
    pub forum_id:       Option<Uuid>,
    pub topic_id:       Option<Uuid>,
    pub post_id:        Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub details:        Option<String>,
    pub created_at:     DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Warning {
    pub id:           Uuid,
    pub user_id:      Uuid,
    pub moderator_id: Uuid,
    pub reason:       String,
    pub created_at:   DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct WarnDto {
    #[validate(length(min = 1, max = 2000))]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Ban {
    pub user_id:    Uuid,
    pub banned_by:  Uuid,
    pub reason:     Option<String>,
    pub until:      Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BanDto {
    pub reason:    Option<String>,
    pub days:      Option<i64>, // None = permanent
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ModNote {
    pub id:             Uuid,
    pub author_id:      Uuid,
    pub target_user_id: Option<Uuid>,
    pub topic_id:       Option<Uuid>,
    pub post_id:        Option<Uuid>,
    pub body:           String,
    pub created_at:     DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ModNoteDto {
    pub target_user_id: Option<Uuid>,
    pub topic_id:       Option<Uuid>,
    pub post_id:        Option<Uuid>,
    #[validate(length(min = 1, max = 4000))]
    pub body:           String,
}
