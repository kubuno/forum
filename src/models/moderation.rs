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
