use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::Result, models::reaction::EmojiAgg};

/// A short allow-list keeps reaction emojis tidy and predictable.
pub const ALLOWED_EMOJIS: &[&str] = &["👍", "❤️", "😂", "😮", "😢", "🎉", "🚀", "👀"];

pub struct ReactionService;

impl ReactionService {
    /// Toggles a reaction. Returns whether the reaction is now present, plus the
    /// post's refreshed per-emoji aggregates for the requesting user.
    pub async fn toggle(
        post_id: Uuid,
        user_id: Uuid,
        emoji: &str,
        db: &PgPool,
    ) -> Result<(bool, Vec<EmojiAgg>)> {
        // The post author earns/loses a "like received"; the reactor a "given".
        let author_id: Uuid = sqlx::query_scalar("SELECT author_id FROM forum.posts WHERE id = $1")
            .bind(post_id)
            .fetch_one(db)
            .await?;

        let mut tx = db.begin().await?;

        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM forum.reactions WHERE post_id = $1 AND user_id = $2 AND emoji = $3",
        )
        .bind(post_id)
        .bind(user_id)
        .bind(emoji)
        .fetch_optional(&mut *tx)
        .await?;

        let added = if let Some(id) = existing {
            sqlx::query("DELETE FROM forum.reactions WHERE id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            false
        } else {
            sqlx::query(
                "INSERT INTO forum.reactions (post_id, user_id, emoji) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(post_id)
            .bind(user_id)
            .bind(emoji)
            .execute(&mut *tx)
            .await?;
            true
        };

        let delta: i64 = if added { 1 } else { -1 };

        // Refresh the denormalised like_count on the post.
        sqlx::query(
            "UPDATE forum.posts SET like_count = (SELECT COUNT(*) FROM forum.reactions WHERE post_id = $1) WHERE id = $1",
        )
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

        // Maintain the two profile counters (create the rows if missing).
        for (uid, col) in [(author_id, "likes_received"), (user_id, "likes_given")] {
            sqlx::query("INSERT INTO forum.user_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
                .bind(uid)
                .execute(&mut *tx)
                .await?;
            sqlx::query(&format!(
                "UPDATE forum.user_profiles SET {col} = GREATEST({col} + $2, 0) WHERE user_id = $1"
            ))
            .bind(uid)
            .bind(delta)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        let aggs = Self::for_post(post_id, user_id, db).await?;
        Ok((added, aggs))
    }

    /// Per-emoji aggregates for one post.
    pub async fn for_post(post_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<EmojiAgg>> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT emoji, COUNT(*) FROM forum.reactions WHERE post_id = $1 GROUP BY emoji ORDER BY COUNT(*) DESC",
        )
        .bind(post_id)
        .fetch_all(db)
        .await?;
        let mine: Vec<String> = sqlx::query_scalar(
            "SELECT emoji FROM forum.reactions WHERE post_id = $1 AND user_id = $2",
        )
        .bind(post_id)
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(emoji, count)| EmojiAgg { me: mine.contains(&emoji), emoji, count })
            .collect())
    }

    /// Per-post aggregates for every post in a topic, for the requesting user.
    pub async fn for_topic(
        topic_id: Uuid,
        user_id: Uuid,
        db: &PgPool,
    ) -> Result<HashMap<Uuid, Vec<EmojiAgg>>> {
        let rows = sqlx::query_as::<_, (Uuid, String, i64)>(
            "SELECT r.post_id, r.emoji, COUNT(*)
             FROM forum.reactions r
             JOIN forum.posts p ON p.id = r.post_id
             WHERE p.topic_id = $1
             GROUP BY r.post_id, r.emoji",
        )
        .bind(topic_id)
        .fetch_all(db)
        .await?;
        let mine = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT r.post_id, r.emoji
             FROM forum.reactions r
             JOIN forum.posts p ON p.id = r.post_id
             WHERE p.topic_id = $1 AND r.user_id = $2",
        )
        .bind(topic_id)
        .bind(user_id)
        .fetch_all(db)
        .await?;
        let mine_set: std::collections::HashSet<(Uuid, String)> = mine.into_iter().collect();

        let mut map: HashMap<Uuid, Vec<EmojiAgg>> = HashMap::new();
        for (post_id, emoji, count) in rows {
            let me = mine_set.contains(&(post_id, emoji.clone()));
            map.entry(post_id).or_default().push(EmojiAgg { emoji, count, me });
        }
        for v in map.values_mut() {
            v.sort_by_key(|b| std::cmp::Reverse(b.count));
        }
        Ok(map)
    }
}
