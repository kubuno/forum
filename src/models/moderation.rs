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
    /// Predefined reason the reporter picked, if any (SEC-15 dedup and the
    /// legacy free-text `reason` both stay independent of this).
    pub reason_id:   Option<Uuid>,
    pub status:      String,
    pub handled_by:  Option<Uuid>,
    pub handled_at:  Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReportDto {
    #[validate(length(min = 1, max = 2000))]
    pub reason: String,
    /// Optional predefined reason (`forum.report_reasons.id`) picked alongside
    /// the free-text comment above.
    #[serde(default)]
    pub reason_id: Option<Uuid>,
}

/// A predefined reason an admin curates for the report chip picker (phpBB-style).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReportReason {
    pub id:          Uuid,
    pub title:       String,
    pub description: Option<String>,
    pub position:    i32,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReportReasonDto {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(max = 2000))]
    pub description: Option<String>,
    #[serde(default)]
    pub position: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResolveReportDto {
    /// New status: 'resolved' or 'rejected'.
    #[validate(length(min = 1, max = 20))]
    pub status: String,
}

/// One row of the approval queue: a message held back by
/// `forum.post_approval_mode`, decorated with enough context (topic title,
/// forum name) that a moderator can decide without opening it.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PendingPost {
    pub id:            Uuid,
    pub topic_id:      Uuid,
    pub forum_id:      Uuid,
    pub author_id:     Uuid,
    pub body_md:       String,
    /// True when releasing this message also releases a whole new topic.
    pub is_first_post: bool,
    pub created_at:    DateTime<Utc>,
    pub topic_title:   String,
    pub forum_name:    String,
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

// ── IP / email bans (phpBB-style, enforced server-side, exact match) ────────

/// A banned IP address (`services::ban_registry`, `middleware::enforce_ban`).
/// `value` is always the exact, canonical `IpAddr::to_string()` form — no
/// CIDR, no wildcard.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct IpBan {
    pub id:         Uuid,
    pub value:      String,
    pub reason:     Option<String>,
    pub banned_by:  Uuid,
    pub until:      Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct IpBanDto {
    #[validate(length(min = 1, max = 64))]
    pub value:  String,
    #[validate(length(max = 2000))]
    pub reason: Option<String>,
    pub days:   Option<i64>, // None = permanent
}

/// A banned email address (always stored lowercase — see
/// `services::moderation_service::ModerationService::ban_email`).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EmailBan {
    pub id:         Uuid,
    pub email:      String,
    pub reason:     Option<String>,
    pub banned_by:  Uuid,
    pub until:      Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct EmailBanDto {
    #[validate(length(min = 1, max = 320))]
    pub email:  String,
    #[validate(length(max = 2000))]
    pub reason: Option<String>,
    pub days:   Option<i64>, // None = permanent
}

// ── Word censor (phpBB-style, applied server-side at render time) ───────────

/// An admin-curated word/phrase substituted in every post body when it is
/// rendered — see `services::censor_service`. `pattern` is always a literal
/// word or phrase, never a regex fragment: the service escapes it
/// (`regex::escape`) before compiling a case-insensitive, word-boundary
/// regex, so an admin can never inject an expensive or malicious pattern.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CensoredWord {
    pub id:          Uuid,
    pub pattern:     String,
    pub replacement: String,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCensoredWordDto {
    #[validate(length(min = 1, max = 200))]
    pub pattern: String,
    /// Defaults to `***` server-side when left empty.
    #[serde(default)]
    #[validate(length(max = 200))]
    pub replacement: Option<String>,
}
