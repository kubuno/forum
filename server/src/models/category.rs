use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id:          Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub position:    i32,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCategoryDto {
    #[validate(length(min = 1, max = 255))]
    pub name:        String,
    #[validate(length(max = 5000))]
    pub description: Option<String>,
    #[serde(default)]
    pub position:    i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCategoryDto {
    #[validate(length(min = 1, max = 255))]
    pub name:        Option<String>,
    #[validate(length(max = 5000))]
    pub description: Option<String>,
    pub position:    Option<i32>,
}
