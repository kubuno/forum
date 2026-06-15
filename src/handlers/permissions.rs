use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ForumUser,
    models::permission::SetPermissionDto,
    services::permission_service::PermissionService,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let permissions = PermissionService::list(forum_id, &state.db).await?;
    Ok(Json(json!({ "permissions": permissions })))
}

pub async fn set(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Json(dto): Json<SetPermissionDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let permission = PermissionService::set(forum_id, dto, &state.db).await?;
    Ok(Json(json!({ "permission": permission })))
}
