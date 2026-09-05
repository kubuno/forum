use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A member's personal RSS/Atom feed credential (see migration `000016`). The
/// `token` is the secret that appears in the feed URL; it is returned to its
/// owner once, on creation and in their own list, and never to anyone else.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FeedToken {
    pub token:        String,
    pub user_id:      Uuid,
    pub label:        Option<String>,
    pub created_at:   DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Body of `POST /me/feed-tokens`: an optional human label ("Reader at home").
#[derive(Debug, Deserialize, Validate)]
pub struct CreateFeedTokenDto {
    #[serde(default)]
    #[validate(length(max = 80))]
    pub label: Option<String>,
}

/// One row rendered into the feed document. Built entirely from topics the
/// token's owner may currently see (see `FeedService::recent_topics_for`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FeedEntry {
    pub topic_id:   Uuid,
    pub forum_id:   Uuid,
    pub title:      String,
    pub author_id:  Uuid,
    pub updated_at: DateTime<Utc>,
}
