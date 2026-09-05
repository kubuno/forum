//! In-memory cache of every currently active ban — by account, IP address,
//! and email — mirroring the pattern of `censor_service`'s
//! `LazyLock<RwLock<...>>`: cheap enough to consult on every request, rebuilt
//! from the database whenever an admin bans/unbans someone (`reload`, called
//! after every write) and once at boot (see `main.rs`).
//!
//! Matching an IP or an email is always an *exact* string comparison — no
//! CIDR, no wildcard, no case-folding beyond the lowercase normalization
//! already applied to emails before they are stored — kept that simple on
//! purpose so the rule stays easy to audit and cannot silently over-match.

use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::Result;

/// The three ban sets, swapped atomically by `reload`.
#[derive(Default)]
struct BanSets {
    users:  HashSet<Uuid>,
    ips:    HashSet<String>,
    emails: HashSet<String>,
}

/// Empty until the first successful reload — every `is_*_banned` call is then
/// a no-op (nobody banned) rather than a panic.
static BANS: LazyLock<RwLock<BanSets>> = LazyLock::new(|| RwLock::new(BanSets::default()));

pub struct BanRegistry;

impl BanRegistry {
    /// Recompiles the in-memory cache from `forum.user_bans`, `forum.ip_bans`
    /// and `forum.email_bans`, keeping only bans that have not expired. Never
    /// blocks the boot sequence or a mutation — a failed reload is logged and
    /// the previous cache (possibly empty) is left in place.
    pub async fn reload(db: &PgPool) -> Result<()> {
        let users: Vec<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM forum.user_bans WHERE until IS NULL OR until > NOW()",
        )
        .fetch_all(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "ban registry: loading user bans failed"))?;

        let ips: Vec<String> = sqlx::query_scalar(
            "SELECT value FROM forum.ip_bans WHERE until IS NULL OR until > NOW()",
        )
        .fetch_all(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "ban registry: loading IP bans failed"))?;

        let emails: Vec<String> = sqlx::query_scalar(
            "SELECT email FROM forum.email_bans WHERE until IS NULL OR until > NOW()",
        )
        .fetch_all(db)
        .await
        .inspect_err(|e| tracing::error!(error = %e, "ban registry: loading email bans failed"))?;

        match BANS.write() {
            Ok(mut guard) => {
                guard.users = users.into_iter().collect();
                guard.ips = ips.into_iter().collect();
                guard.emails = emails.into_iter().collect();
            }
            Err(e) => tracing::error!(error = %e, "ban registry: lock poisoned on write"),
        }
        Ok(())
    }

    /// True if `user_id` is currently banned (respecting expiry, as of the
    /// last reload).
    pub fn is_user_banned(user_id: Uuid) -> bool {
        match BANS.read() {
            Ok(guard) => guard.users.contains(&user_id),
            Err(e) => {
                tracing::error!(error = %e, "ban registry: lock poisoned on read");
                false
            }
        }
    }

    /// True if `ip` (expected already in the canonical `IpAddr::to_string()`
    /// form the core forwards) exactly matches a currently active IP ban.
    pub fn is_ip_banned(ip: &str) -> bool {
        match BANS.read() {
            Ok(guard) => guard.ips.contains(ip),
            Err(e) => {
                tracing::error!(error = %e, "ban registry: lock poisoned on read");
                false
            }
        }
    }

    /// True if `email` (case-insensitively) exactly matches a currently
    /// active email ban.
    pub fn is_email_banned(email: &str) -> bool {
        let email = email.trim().to_lowercase();
        match BANS.read() {
            Ok(guard) => guard.emails.contains(&email),
            Err(e) => {
                tracing::error!(error = %e, "ban registry: lock poisoned on read");
                false
            }
        }
    }
}
