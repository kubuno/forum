use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ForumUser,
    services::notification_service::NotificationService,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub unread: Option<bool>,
    pub limit:  Option<i64>,
}

/// GET /me/notifications
pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>> {
    let items = NotificationService::list(user.id, q.unread.unwrap_or(false), q.limit.unwrap_or(50), &state.db).await?;
    let unread = NotificationService::unread_count(user.id, &state.db).await?;
    Ok(Json(json!({ "notifications": items, "unread": unread })))
}

/// POST /me/notifications/read  (mark one or all read)
#[derive(Deserialize)]
pub struct MarkDto {
    pub id: Option<Uuid>,
}

pub async fn mark_read(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<MarkDto>,
) -> Result<Json<Value>> {
    if let Some(id) = dto.id {
        NotificationService::mark_read(user.id, id, &state.db).await?;
    } else {
        NotificationService::mark_all_read(user.id, &state.db).await?;
    }
    let unread = NotificationService::unread_count(user.id, &state.db).await?;
    Ok(Json(json!({ "unread": unread })))
}
