//! Ignore list (phpBB-style "foes"/zebra): a member can ignore another member
//! so their posts fold under a "hidden" banner in the topic view (see
//! migration `000015`). Purely a display preference — see
//! `services/ignore_service.rs` for the security model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct IgnoredUser {
    pub user_id:         Uuid,
    pub ignored_user_id: Uuid,
    pub created_at:      DateTime<Utc>,
}

/// `POST /me/ignored` — the member to ignore.
#[derive(Debug, Deserialize)]
pub struct IgnoreDto {
    pub user_id: Uuid,
}
