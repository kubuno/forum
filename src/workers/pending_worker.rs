//! Approval-queue cleaner: discards contributions nobody ever decided on.
//!
//! A queue that only grows is not moderation, it is a backlog: the messages sit
//! invisible forever, their authors never learn what happened, and the counter
//! on the moderation entry stops meaning anything. The retention window comes
//! from the admin console (`forum.pending_retention_days`), is re-read on every
//! pass, and `0` turns the sweep off — an administrator must be able to say
//! "nothing is ever discarded automatically".
//!
//! Only UNAPPROVED rows are touched. Nothing published is ever at risk here, and
//! because pending rows were never counted in the denormalised aggregates,
//! removing them leaves every counter already correct.

use std::time::Duration;

use crate::services::moderation_service::ModerationService;
use crate::state::AppState;

/// Retention is measured in days, so an hourly pass is precise enough and cheap.
const SWEEP_INTERVAL: Duration = Duration::from_secs(3600);

pub async fn start(state: AppState) {
    loop {
        tokio::time::sleep(SWEEP_INTERVAL).await;

        let days = state.instance().pending_retention_days;
        if days <= 0 {
            continue; // retention disabled: the queue is kept indefinitely
        }

        match ModerationService::purge_stale_pending(days, &state.db).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(discarded = n, retention_days = days, "Stale pending contributions discarded"),
            Err(e) => tracing::error!(error = %e, "Approval-queue cleaner"),
        }
    }
}
