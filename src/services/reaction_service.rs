use std::collections::HashMap;

use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::{errors::Result, models::reaction::{EmojiAgg, ReactionUsers}};

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
        db: &DbPool,
    ) -> Result<(bool, Vec<EmojiAgg>)> {
        // The post author earns/loses a "like received"; the reactor a "given".
        let author_id: Uuid = db
            .fetch_scalar("SELECT author_id FROM forum.posts WHERE id = $1", params![post_id])
            .await?;

        let mut tx = db.begin().await?;

        let existing: Option<Uuid> = tx
            .fetch_optional_scalar(
                "SELECT id FROM forum.reactions WHERE post_id = $1 AND user_id = $2 AND emoji = $3",
                params![post_id, user_id, emoji],
            )
            .await?;

        let b = db.backend();
        let added = if let Some(id) = existing {
            tx.execute("DELETE FROM forum.reactions WHERE id = $1", params![id]).await?;
            false
        } else {
            let sql = format!(
                "INSERT {}INTO forum.reactions (id, post_id, user_id, emoji) VALUES ($1, $2, $3, $4){}",
                b.insert_ignore_prefix(),
                b.on_conflict_do_nothing(&["post_id", "user_id", "emoji"]),
            );
            tx.execute(&sql, params![new_id(), post_id, user_id, emoji]).await?;
            true
        };

        // like_count is an INT column: bind the delta at that width.
        let delta: i32 = if added { 1 } else { -1 };

        // Refresh the denormalised like_count on the post (distinct placeholders,
        // never reused: SqlSafeStr).
        tx.execute(
            "UPDATE forum.posts SET like_count = \
                (SELECT COUNT(*) FROM forum.reactions WHERE post_id = $1) WHERE id = $2",
            params![post_id, post_id],
        )
        .await?;

        // Maintain the two profile counters (create the rows if missing).
        let ensure = format!(
            "INSERT {}INTO forum.user_profiles (user_id) VALUES ($1){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(&["user_id"]),
        );
        tx.execute(&ensure, params![author_id]).await?;
        tx.execute(&ensure, params![user_id]).await?;
        // GREATEST has no SQLite equivalent: floor at 0 with a portable CASE.
        tx.execute(
            "UPDATE forum.user_profiles \
                SET likes_received = CASE WHEN likes_received + $1 < 0 THEN 0 ELSE likes_received + $2 END \
              WHERE user_id = $3",
            params![delta, delta, author_id],
        )
        .await?;
        tx.execute(
            "UPDATE forum.user_profiles \
                SET likes_given = CASE WHEN likes_given + $1 < 0 THEN 0 ELSE likes_given + $2 END \
              WHERE user_id = $3",
            params![delta, delta, user_id],
        )
        .await?;

        tx.commit().await?;

        let aggs = Self::for_post(post_id, user_id, db).await?;
        Ok((added, aggs))
    }

    /// Per-emoji aggregates for one post.
    pub async fn for_post(post_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<EmojiAgg>> {
        let rows = db
            .fetch_all_as::<(String, i64)>(
                "SELECT emoji, COUNT(*) FROM forum.reactions WHERE post_id = $1 GROUP BY emoji ORDER BY COUNT(*) DESC",
                params![post_id],
            )
            .await?;
        let mine: Vec<String> = db
            .fetch_all_as::<(String,)>(
                "SELECT emoji FROM forum.reactions WHERE post_id = $1 AND user_id = $2",
                params![post_id, user_id],
            )
            .await?
            .into_iter()
            .map(|(e,)| e)
            .collect();
        Ok(rows
            .into_iter()
            .map(|(emoji, count)| EmojiAgg { me: mine.contains(&emoji), emoji, count })
            .collect())
    }

    /// Per-post aggregates for every post in a topic, for the requesting user.
    pub async fn for_topic(
        topic_id: Uuid,
        user_id: Uuid,
        db: &DbPool,
    ) -> Result<HashMap<Uuid, Vec<EmojiAgg>>> {
        let rows = db
            .fetch_all_as::<(Uuid, String, i64)>(
                "SELECT r.post_id, r.emoji, COUNT(*) \
                 FROM forum.reactions r \
                 JOIN forum.posts p ON p.id = r.post_id \
                 WHERE p.topic_id = $1 \
                 GROUP BY r.post_id, r.emoji",
                params![topic_id],
            )
            .await?;
        let mine = db
            .fetch_all_as::<(Uuid, String)>(
                "SELECT r.post_id, r.emoji \
                 FROM forum.reactions r \
                 JOIN forum.posts p ON p.id = r.post_id \
                 WHERE p.topic_id = $1 AND r.user_id = $2",
                params![topic_id, user_id],
            )
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

    /// Who reacted to one post, grouped by emoji — up to 50 users per emoji
    /// (oldest reaction first), for the "who reacted" tooltip.
    pub async fn users_for_post(post_id: Uuid, db: &DbPool) -> Result<Vec<ReactionUsers>> {
        let rows = db
            .fetch_all_as::<(String, Uuid)>(
                "SELECT emoji, user_id FROM ( \
                    SELECT emoji, user_id, \
                           ROW_NUMBER() OVER (PARTITION BY emoji ORDER BY created_at ASC) AS rn \
                      FROM forum.reactions WHERE post_id = $1 \
                 ) ranked \
                 WHERE rn <= 50 \
                 ORDER BY emoji, rn",
                params![post_id],
            )
            .await?;

        let mut map: HashMap<String, Vec<Uuid>> = HashMap::new();
        for (emoji, user_id) in rows {
            map.entry(emoji).or_default().push(user_id);
        }
        let mut out: Vec<ReactionUsers> = map
            .into_iter()
            .map(|(emoji, users)| ReactionUsers { emoji, users })
            .collect();
        out.sort_by(|a, b| a.emoji.cmp(&b.emoji));
        Ok(out)
    }
}
