use axum::{
    extract::{Path, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::ForumUser,
    models::permission::{SetGroupPermissionDto, SetPermissionDto},
    services::{directory, moderation_service::ModerationService, permission_service::PermissionService},
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
    ModerationService::log(
        user.id, "forum_permissions_changed", Some(forum_id), None, None, None,
        Some(&permission.role), &state.db,
    ).await;
    Ok(Json(json!({ "permission": permission })))
}

/// GET /groups — the instance's user groups (id + name), so the admin panel can
/// offer them when granting per-group forum access. Admin only.
pub async fn list_groups(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let groups = directory::list_groups(&state).await;
    Ok(Json(json!({ "groups": groups })))
}

/// GET /forums/:id/group-permissions — the per-group grants on a forum.
pub async fn list_group(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let permissions = PermissionService::list_group(forum_id, &state.db).await?;
    Ok(Json(json!({ "permissions": permissions })))
}

/// PUT /forums/:id/group-permissions — upsert (or clear) one group's grant.
pub async fn set_group(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(forum_id): Path<Uuid>,
    Json(dto): Json<SetGroupPermissionDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    let group_id = dto.group_id;
    PermissionService::set_group(forum_id, dto, &state.db).await?;
    ModerationService::log(
        user.id, "forum_group_permissions_changed", Some(forum_id), None, None, None,
        Some(&group_id.to_string()), &state.db,
    ).await;
    let permissions = PermissionService::list_group(forum_id, &state.db).await?;
    Ok(Json(json!({ "permissions": permissions })))
}
