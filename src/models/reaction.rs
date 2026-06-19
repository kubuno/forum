use serde::{Deserialize, Serialize};

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
