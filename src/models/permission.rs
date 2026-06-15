use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id:         Uuid,
    pub forum_id:   Uuid,
    pub role:       String,
    pub can_view:   bool,
    pub can_post:   bool,
    pub can_reply:  bool,
    pub can_attach: bool,
}

/// Upsert a permission row for a (forum, role) pair.
#[derive(Debug, Deserialize)]
pub struct SetPermissionDto {
    pub role:       String,
    pub can_view:   bool,
    pub can_post:   bool,
    pub can_reply:  bool,
    pub can_attach: bool,
}
