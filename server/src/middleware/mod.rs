use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{errors::ForumError, services::ban_registry::BanRegistry, state::AppState};

mod rate_limit;
pub use rate_limit::{rate_limit_writes, WriteRateLimiter};

/// User extracted from the headers injected by the core.
#[derive(Debug, Clone)]
pub struct ForumUser {
    pub id:    Uuid,
    pub role:  String,
    pub email: String,
    /// Core user-group ids the caller belongs to, resolved at authentication
    /// (see `services::directory::user_group_ids`). Drives the ADDITIVE
    /// per-group forum permissions; empty when the caller is in no group or the
    /// core could not be reached (fail-closed — never widens access spuriously).
    pub group_ids: Vec<Uuid>,
}

impl ForumUser {
    /// Whether the user is a platform administrator.
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Axum extension key to store the user in the request.
pub type ForumUserExt = axum::Extension<ForumUser>;

/// Middleware: extracts X-Kubuno-User-Id, X-Kubuno-User-Role, X-Kubuno-User-Email.
/// These headers are injected by the core proxy — they are trusted.
/// This module's id, used as the token audience.
const MODULE_ID: &str = "forum";

/// Authenticate the caller from the signed `X-Kubuno-Auth` token the core
/// mints with this module's internal secret (see `kubuno-modauth`), instead of
/// trusting the plain `X-Kubuno-User-*` headers — which any process reaching this
/// module's loopback port could forge to impersonate any user.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, ForumError> {
    let token = req
        .headers()
        .get(kubuno_modauth::TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(ForumError::Unauthorized)?;

    let user = kubuno_modauth::verify(
        state.settings.core.internal_secret.as_bytes(),
        token,
        MODULE_ID,
    )
    .map_err(|_| ForumError::Unauthorized)?;

    // Resolve the caller's group memberships (cached; fail-closed to empty) so
    // the additive per-group forum permissions can be applied downstream.
    let group_ids = crate::services::directory::user_group_ids(&state, user.id).await;

    req.extensions_mut()
        .insert(ForumUser { id: user.id, role: user.role, email: user.email, group_ids });
    Ok(next.run(req).await)
}

/// Header the core injects with the caller's real IP, hardened against
/// spoofing on its side (`crate::auth::client_ip` in the core) — see
/// `kubuno_core::modules::proxy`. Absent for a module→module call.
const CLIENT_IP_HEADER: &str = "x-kubuno-client-ip";

/// Rejects a request from someone currently banned by account, IP, or email
/// (block lists — see `services::ban_registry`). Must run
/// *after* `require_auth` (see `router::build`), so `ForumUser` is already in
/// the request extensions; consults only the in-memory `BanRegistry`, never
/// the database, so it costs nothing on the hot path.
///
/// An administrator is never blocked here — a ban set by mistake on an admin
/// account must not lock them out with no way back short of a database edit,
/// mirroring `ModerationService::ban` refusing to let a moderator ban
/// themselves.
pub async fn enforce_ban(req: Request, next: Next) -> std::result::Result<Response, ForumError> {
    if let Some(user) = req.extensions().get::<ForumUser>() {
        if !user.is_admin() {
            let ip_banned = req
                .headers()
                .get(CLIENT_IP_HEADER)
                .and_then(|v| v.to_str().ok())
                .is_some_and(BanRegistry::is_ip_banned);
            if ip_banned
                || BanRegistry::is_user_banned(user.id)
                || BanRegistry::is_email_banned(&user.email)
            {
                return Err(ForumError::Forbidden);
            }
        }
    }
    Ok(next.run(req).await)
}
