pub mod attachments;
pub mod bookmarks;
pub mod categories;
pub mod community;
pub mod discovery;
pub mod drafts;
pub mod faq;
pub mod feed;
pub mod forums;
pub mod health;
pub mod ignore;
pub mod moderation;
pub mod notifications;
pub mod permissions;
pub mod pm;
pub mod polls;
pub mod posts;
pub mod profile_fields;
pub mod ranks;
pub mod reactions;
pub mod search;
pub mod tags;
pub mod topics;

use serde::Deserialize;
use uuid::Uuid;

use crate::{
    config::instance::InstanceConfig,
    errors::{ForumError, Result},
    services::rank_service::RankService,
    state::AppState,
};

/// Refuses a message longer than the instance allows.
///
/// The DTOs carry a compile-time `#[validate(length(max = 100000))]` ceiling,
/// which is a sanity bound on the wire format; this is the ADMINISTRATOR's
/// ceiling, which is a policy and can be lowered from the console. Both apply,
/// and the stricter one wins — as it should.
pub fn assert_body_within_limit(body: &str, cfg: &InstanceConfig) -> Result<()> {
    if body.chars().count() > cfg.max_post_length {
        return Err(ForumError::Validation(format!(
            "message trop long ({} caractères maximum)",
            cfg.max_post_length
        )));
    }
    Ok(())
}

/// Whether a contribution by this author is published immediately or held in
/// the approval queue.
///
/// The author's post count is only fetched when the queue is actually armed —
/// on the overwhelming majority of instances, where nothing is moderated, this
/// costs no query at all.
pub async fn approval_decision(
    state: &AppState,
    author_id: Uuid,
    is_moderator: bool,
    cfg: &InstanceConfig,
) -> Result<bool> {
    if is_moderator || !cfg.post_approval_mode.is_active() {
        return Ok(true);
    }
    let profile = RankService::get_profile(author_id, &state.db).await?;
    Ok(cfg.post_approval_mode.publishes_immediately(
        false,
        profile.post_count,
        cfg.new_member_post_count,
    ))
}

/// Shared pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

impl Pagination {
    /// Clamp the limit to a sane range and default the offset.
    pub fn resolve(&self, default: i64, max: i64) -> (i64, i64) {
        let limit = self.limit.unwrap_or(default).clamp(1, max);
        let offset = self.offset.unwrap_or(0).max(0);
        (limit, offset)
    }
}
