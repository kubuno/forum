use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Aggregated reactions of one emoji on a post.
#[derive(Debug, Clone, Serialize)]
pub struct EmojiAgg {
    pub emoji: String,
    pub count: i64,
    /// Whether the requesting user reacted with this emoji.
    pub me:    bool,
}

#[derive(Debug, Deserialize)]
pub struct ReactDto {
    pub emoji: String,
}

/// The users who reacted to a post with a given emoji — feeds the "who
/// reacted" tooltip, loaded on demand rather than for every post at once.
#[derive(Debug, Clone, Serialize)]
pub struct ReactionUsers {
    pub emoji: String,
    pub users: Vec<Uuid>,
}
