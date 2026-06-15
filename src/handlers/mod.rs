pub mod attachments;
pub mod categories;
pub mod forums;
pub mod health;
pub mod moderation;
pub mod permissions;
pub mod posts;
pub mod ranks;
pub mod search;
pub mod topics;

use serde::Deserialize;

/// Shared pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

impl Pagination {
    /// Clamp the limit to a sane range and default the offset.
    pub fn resolve(&self, default: i64, max: i64) -> (i64, i64) {
        let limit = self.limit.unwrap_or(default).clamp(1, max);
        let offset = self.offset.unwrap_or(0).max(0);
        (limit, offset)
    }
}
