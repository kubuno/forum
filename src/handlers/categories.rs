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
    models::category::{CreateCategoryDto, UpdateCategoryDto},
    services::{category_service::CategoryService, permission_service::PermissionService},
    state::AppState,
};

pub async fn list(State(state): State<AppState>) -> Result<Json<Value>> {
    let categories = CategoryService::list(&state.db).await?;
    Ok(Json(json!({ "categories": categories })))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    let category = CategoryService::get(id, &state.db).await?;
    Ok(Json(json!({ "category": category })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<CreateCategoryDto>,
) -> Result<(StatusCode, Json<Value>)> {
    PermissionService::assert_admin(&user)?;
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let category = CategoryService::create(dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(json!({ "category": category }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateCategoryDto>,
) -> Result<Json<Value>> {
    PermissionService::assert_admin(&user)?;
    dto.validate().map_err(|e| ForumError::Validation(e.to_string()))?;
    let category = CategoryService::update(id, dto, &state.db).await?;
    Ok(Json(json!({ "category": category })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    PermissionService::assert_admin(&user)?;
    CategoryService::delete(id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
