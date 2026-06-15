use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Rank {
    pub id:         Uuid,
    pub title:      String,
    pub min_posts:  i32,
    pub is_special: bool,
    pub badge:      Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRankDto {
    #[validate(length(min = 1, max = 100))]
    pub title:      String,
    #[serde(default)]
    pub min_posts:  i32,
    #[serde(default)]
    pub is_special: bool,
    #[validate(length(max = 40))]
    pub badge:      Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRankDto {
    #[validate(length(min = 1, max = 100))]
    pub title:      Option<String>,
    pub min_posts:  Option<i32>,
    pub is_special: Option<bool>,
    #[validate(length(max = 40))]
    pub badge:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserProfile {
    pub user_id:      Uuid,
    pub post_count:   i32,
    pub rank_id:      Option<Uuid>,
    pub signature_md: Option<String>,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileDto {
    #[validate(length(max = 2000))]
    pub signature_md: Option<String>,
}
