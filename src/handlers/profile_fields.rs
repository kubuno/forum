use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::profile_field::{CreateProfileFieldDto, SetProfileValuesDto, UpdateProfileFieldDto},
    services::{
        moderation_service::ModerationService, permission_service::PermissionService,
        profile_field_service::ProfileFieldService,
    },
    state::AppState,
};

// ── Definitions (admin-curated) ──────────────────────────────────────────────

/// GET /profile-fields — every authenticated member sees the definitions, so
/// a profile page or an edit form can render the right control for each one.
pub async fn list_fields(State(state): State<AppState>) -> Result<Json<Value>> {
    let fields = ProfileFieldService::list_fields(&state.db).await?;
    Ok(Json(json!({ "fields": fields })))
}

pub async fn create_field(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateProfileFieldDto>,
) -> Result<(StatusCode, Json<Value>)> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let field = ProfileFieldService::create_field(dto, &state.db).await?;
    ModerationService::log(user.id, "profile_field_added", None, None, None, None, Some(&field.key), &state.db).await;
    Ok((StatusCode::CREATED, Json(json!({ "field": field }))))
}

pub async fn update_field(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateProfileFieldDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    PermissionService::assert_admin(&user)?;
    let field = ProfileFieldService::update_field(id, dto, &state.db).await?;
    ModerationService::log(user.id, "profile_field_updated", None, None, None, None, Some(&field.key), &state.db).await;
    Ok(Json(json!({ "field": field })))
}

pub async fn delete_field(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    ProfileFieldService::delete_field(id, &state.db).await?;
    ModerationService::log(user.id, "profile_field_removed", None, None, None, None, None, &state.db).await;
    Ok(StatusCode::NO_CONTENT)
}

// ── Values (per member) ───────────────────────────────────────────────────────

/// GET /me/profile-fields — the caller's own answers, alongside every
/// definition (so the edit form can render a control per field even for one
/// the caller has never answered).
pub async fn my_values(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let fields = ProfileFieldService::list_fields(&state.db).await?;
    let values = ProfileFieldService::get_values(user.id, &state.db).await?;
    Ok(Json(json!({ "fields": fields, "values": values })))
}

/// PUT /me/profile-fields — set (or clear) the caller's own answers. A member
/// may only ever write their own row: there is no `user_id` in the DTO, it
/// always comes from the authenticated caller.
pub async fn set_my_values(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<SetProfileValuesDto>,
) -> Result<Json<Value>> {
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let values = ProfileFieldService::set_values(user.id, dto, &state.db).await?;
    Ok(Json(json!({ "values": values })))
}

/// GET /users/:id/profile-fields — a member's answers, shaped for the
/// profile page: only the (field, value) pairs the viewer is allowed to see,
/// and only for fields that actually have an answer.
pub async fn user_values(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Value>> {
    let pairs = ProfileFieldService::values_for_display(uid, Some(&user), &state.db).await?;
    let fields: Vec<_> = pairs
        .into_iter()
        .map(|(field, value)| json!({ "field": field, "value": value }))
        .collect();
    Ok(Json(json!({ "fields": fields })))
}
