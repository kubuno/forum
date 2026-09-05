use crate::config::instance::InstanceConfig;
use crate::config::Settings;
use crate::middleware::WriteRateLimiter;
use sqlx::PgPool;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub db:       PgPool,
    pub settings: Arc<Settings>,
    /// Instance settings from the admin console, refreshed in the background so
    /// an edit takes effect without restarting the module. Read through
    /// [`AppState::instance`], never locked directly by callers.
    pub instance: Arc<RwLock<InstanceConfig>>,
    /// Per-user write rate limiter, shared across every request (SEC-09).
    pub rate_limiter: WriteRateLimiter,
}

impl AppState {
    /// A snapshot of the current instance settings. Falls back to the compiled
    /// defaults if the lock was poisoned by a panicking writer — a lost value
    /// must never take a protection down.
    pub fn instance(&self) -> InstanceConfig {
        self.instance.read().map(|c| *c).unwrap_or_default()
    }
}
