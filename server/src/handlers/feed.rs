//! RSS/Atom feeds.
//!
//! Public, anonymous routes (`/public/feeds/:token/...`) are the ONLY ones the
//! router leaves outside `require_auth`; they authenticate purely on the
//! capability token in the URL. The management routes (`/me/feed-tokens`) are
//! behind auth like everything else. See `services/feed_service.rs` for the
//! security model.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::{json, Value};
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::feed::CreateFeedTokenDto,
    services::feed_service::FeedService,
    state::AppState,
};

enum FeedKind {
    Atom,
    Rss,
}

/// Reconstructs the public origin the reader used, but ONLY from
/// `X-Forwarded-Host` — a forwarding proxy chain that means to expose the feed
/// externally sets it deliberately. The plain `Host` header is useless here: by
/// the time the request reaches this module it names the core's loopback target
/// (`127.0.0.1:<port>`), never the public site. When no forwarded host is
/// present we return an empty base, so the feed emits ROOT-RELATIVE links
/// (`/forum/...`) — a feed reader resolves those against the feed's own URL,
/// which is already the correct public origin, so they end up right regardless.
fn base_from_headers(headers: &HeaderMap) -> String {
    let host = headers
        .get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|h| !h.is_empty());
    match host {
        Some(h) => {
            let proto = headers
                .get("x-forwarded-proto")
                .and_then(|v| v.to_str().ok())
                .filter(|p| !p.is_empty())
                .unwrap_or("https");
            format!("{proto}://{h}")
        }
        None => String::new(),
    }
}

async fn serve_feed(state: &AppState, token: &str, headers: &HeaderMap, kind: FeedKind) -> Result<Response> {
    // Unknown token → 404, indistinguishable from any other miss.
    let user_id = FeedService::resolve(token, &state.db)
        .await?
        .ok_or_else(|| ForumError::NotFound("flux introuvable".into()))?;

    let entries = FeedService::recent_topics_for(state, user_id).await?;
    let base = base_from_headers(headers);

    let (body, content_type) = match kind {
        FeedKind::Atom => (
            FeedService::render_atom(state, &base, &entries).await,
            "application/atom+xml; charset=utf-8",
        ),
        FeedKind::Rss => (
            FeedService::render_rss(state, &base, &entries).await,
            "application/rss+xml; charset=utf-8",
        ),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        // Feed readers poll often; let the proxy/reader cache briefly.
        .header(header::CACHE_CONTROL, "private, max-age=300")
        .body(Body::from(body))
        .map_err(|e| ForumError::Internal(e.into()))
}

/// GET /public/feeds/:token/atom.xml — anonymous, token-authenticated.
pub async fn public_atom(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    serve_feed(&state, &token, &headers, FeedKind::Atom).await
}

/// GET /public/feeds/:token/rss.xml — anonymous, token-authenticated.
pub async fn public_rss(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    serve_feed(&state, &token, &headers, FeedKind::Rss).await
}

// ── Token management (authenticated) ─────────────────────────────────────────

/// GET /me/feed-tokens — the caller's own feed URLs.
pub async fn list_tokens(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let tokens = FeedService::list_tokens(user.id, &state.db).await?;
    Ok(Json(json!({ "tokens": tokens })))
}

/// POST /me/feed-tokens — mints a new feed URL.
pub async fn create_token(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateFeedTokenDto>,
) -> Result<impl IntoResponse> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let token = FeedService::create_token(user.id, dto.label, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "token": token }))))
}

/// DELETE /me/feed-tokens/:token — revokes one of the caller's feed URLs.
pub async fn revoke_token(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(token): Path<String>,
) -> Result<StatusCode> {
    FeedService::revoke_token(user.id, &token, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
