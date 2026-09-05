//! Private messages: modern 1:1 / small-group conversations (see migration
//! `000013`). Kept fully separate from the phpBB-style boards — no folders,
//! no rules engine — and from search (`search_service` never touches these
//! tables: a DM is never indexed).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PmThread {
    pub id:              Uuid,
    pub subject:         Option<String>,
    pub created_by:      Uuid,
    pub created_at:      DateTime<Utc>,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PmMessage {
    pub id:         Uuid,
    pub thread_id:  Uuid,
    pub sender_id:  Uuid,
    pub body_md:    String,
    pub created_at: DateTime<Utc>,
}

/// Starts a new thread with its first message, in one call.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateThreadDto {
    /// The other participants. Capped well below "broadcast" territory —
    /// this is a conversation, not a mailing list.
    #[validate(length(min = 1, max = 10))]
    pub recipient_ids: Vec<Uuid>,
    #[validate(length(max = 300))]
    pub subject:       Option<String>,
    #[validate(length(min = 1, max = 100000))]
    pub body_md:       String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageDto {
    #[validate(length(min = 1, max = 100000))]
    pub body_md: String,
}

/// `POST /me/pm/blocks` — the target to block. `DELETE /me/pm/blocks/:uid`
/// takes the id from the path instead and does not use this DTO.
#[derive(Debug, Deserialize)]
pub struct BlockDto {
    pub user_id: Uuid,
}

/// One row of `GET /me/pm` — enough to render the inbox list without a
/// second round trip per thread.
#[derive(Debug, Serialize)]
pub struct ThreadSummary {
    pub id:                    Uuid,
    pub subject:               Option<String>,
    pub participant_ids:       Vec<Uuid>,
    pub last_message_at:       DateTime<Utc>,
    /// Truncated body of the most recent message, or `None` for a thread
    /// that somehow has none (never happens in practice: creation always
    /// inserts a first message).
    pub last_message_preview:  Option<String>,
    /// Messages in this thread authored by someone else, newer than the
    /// caller's `last_read_at`. Zero means read.
    pub unread_count:          i64,
}

/// `GET /me/pm/:id` thread header, per the contract — deliberately narrower
/// than [`ThreadSummary`] (no preview/unread — the caller is about to read
/// the messages themselves).
#[derive(Debug, Serialize)]
pub struct ThreadDetail {
    pub id:              Uuid,
    pub subject:         Option<String>,
    pub participant_ids: Vec<Uuid>,
}
