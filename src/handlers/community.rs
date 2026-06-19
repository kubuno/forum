use axum::{extract::State, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    errors::Result,
    middleware::ForumUser,
    services::{presence_service::PresenceService, stats_service::StatsService},
    state::AppState,
};

#[derive(Deserialize)]
pub struct HeartbeatDto {
    pub path: Option<String>,
}

/// POST /me/heartbeat — keeps the user listed as online.
pub async fn heartbeat(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<HeartbeatDto>,
) -> Result<Json<Value>> {
    PresenceService::heartbeat(user.id, dto.path.as_deref(), &state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

/// GET /online — user ids active in the last 5 minutes.
pub async fn online(State(state): State<AppState>) -> Result<Json<Value>> {
    let ids = PresenceService::who_online(5, &state.db).await?;
    Ok(Json(json!({ "user_ids": ids })))
}

/// GET /stats — community-wide totals.
pub async fn stats(State(state): State<AppState>) -> Result<Json<Value>> {
    let stats = StatsService::global(&state.db).await?;
    Ok(Json(json!({ "stats": stats })))
}

/// GET /members — latest and top members.
pub async fn members(State(state): State<AppState>) -> Result<Json<Value>> {
    let latest = StatsService::latest_members(10, &state.db).await?;
    let top = StatsService::top_posters(10, &state.db).await?;
    Ok(Json(json!({
        "latest": latest,
        "top": top.into_iter().map(|(id, n)| json!({ "user_id": id, "post_count": n })).collect::<Vec<_>>(),
    })))
}
