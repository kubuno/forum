use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{errors::ForumError, state::AppState};

/// User extracted from the headers injected by the core.
#[derive(Debug, Clone)]
pub struct ForumUser {
    pub id:    Uuid,
    pub role:  String,
    pub email: String,
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
pub async fn require_auth(
    State(_state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, ForumError> {
    let user_id = req
        .headers()
        .get("x-kubuno-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(ForumError::Unauthorized)?;

    let role = req
        .headers()
        .get("x-kubuno-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user")
        .to_string();

    let email = req
        .headers()
        .get("x-kubuno-user-email")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    req.extensions_mut()
        .insert(ForumUser { id: user_id, role, email });
    Ok(next.run(req).await)
}
