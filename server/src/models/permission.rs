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

/// A per-GROUP forum permission (see migration `000019`). `group_id` is a core
/// user-group id. These grants are ADDITIVE on top of the role-based rules.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GroupPermission {
    pub id:         Uuid,
    pub forum_id:   Uuid,
    pub group_id:   Uuid,
    pub can_view:   bool,
    pub can_post:   bool,
    pub can_reply:  bool,
    pub can_attach: bool,
}

/// Upsert (or, when every flag is false, clear) a per-group grant on a forum.
#[derive(Debug, Deserialize)]
pub struct SetGroupPermissionDto {
    pub group_id:   Uuid,
    pub can_view:   bool,
    pub can_post:   bool,
    pub can_reply:  bool,
    pub can_attach: bool,
}
