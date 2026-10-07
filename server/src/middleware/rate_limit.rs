use std::{num::NonZeroU32, sync::Arc};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use governor::{
    clock::{Clock, DefaultClock},
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use uuid::Uuid;

use crate::{errors::ForumError, middleware::ForumUser, state::AppState};

/// A single-key GCRA limiter — one token bucket.
type UserBucket = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Per-user write rate limiting (SEC-09).
///
/// Each authenticated user gets their own GCRA bucket, held in a sharded
/// concurrent map so simultaneous requests from one user share a single bucket
/// without a global lock. Reads are never limited; only state-changing requests
/// are checked. Buckets accumulate one entry per user seen since start-up, which
/// is bounded by the member count of a self-hosted instance; a periodic sweep of
/// idle buckets can be added later if that ever matters.
#[derive(Clone)]
pub struct WriteRateLimiter {
    quota:   Quota,
    clock:   DefaultClock,
    buckets: Arc<DashMap<Uuid, Arc<UserBucket>>>,
}

impl WriteRateLimiter {
    /// At most `per_minute` write requests per user per minute (floored at 1).
    pub fn per_minute(per_minute: u32) -> Self {
        let max = NonZeroU32::new(per_minute.max(1)).expect("floored at 1");
        Self {
            quota:   Quota::per_minute(max),
            clock:   DefaultClock::default(),
            buckets: Arc::new(DashMap::new()),
        }
    }

    /// `Ok(())` while the user is within budget; `Err(retry_after_seconds)` once
    /// their bucket is exhausted.
    pub fn check(&self, user: Uuid) -> std::result::Result<(), u64> {
        let bucket = self
            .buckets
            .entry(user)
            .or_insert_with(|| Arc::new(RateLimiter::direct(self.quota)))
            .clone();
        match bucket.check() {
            Ok(()) => Ok(()),
            Err(neg) => Err(neg.wait_time_from(self.clock.now()).as_secs().max(1)),
        }
    }
}

/// Middleware: rejects a user who exceeds the write budget with `429`. Safe
/// methods (GET/HEAD/OPTIONS/TRACE) pass through untouched. It runs *after*
/// `require_auth`, so the `ForumUser` is already in the request extensions; an
/// unauthenticated request (none present) is left for `require_auth` to reject.
pub async fn rate_limit_writes(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> std::result::Result<Response, ForumError> {
    if req.method().is_safe() {
        return Ok(next.run(req).await);
    }
    if let Some(user) = req.extensions().get::<ForumUser>() {
        if let Err(retry_after) = state.rate_limiter.check(user.id) {
            return Err(ForumError::RateLimited(retry_after as i64));
        }
    }
    Ok(next.run(req).await)
}
