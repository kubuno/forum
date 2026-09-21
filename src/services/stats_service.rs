use chrono::{Duration, Utc};
use kubuno_db::{params, DbPool};
use serde::Serialize;
use uuid::Uuid;

use crate::errors::Result;

#[derive(Debug, Serialize)]
pub struct ForumStats {
    pub categories: i64,
    pub forums:     i64,
    pub topics:     i64,
    pub posts:      i64,
    pub members:    i64,
    pub reactions:  i64,
    pub online:     i64,
    pub latest_member: Option<Uuid>,
}

pub struct StatsService;

impl StatsService {
    pub async fn global(db: &DbPool) -> Result<ForumStats> {
        let count = |sql: &'static str| async move { db.fetch_scalar::<i64>(sql, params![]).await };
        let categories = count("SELECT COUNT(*) FROM forum.categories").await?;
        let forums     = count("SELECT COUNT(*) FROM forum.forums").await?;
        let topics     = count("SELECT COUNT(*) FROM forum.topics").await?;
        let posts      = count("SELECT COUNT(*) FROM forum.posts WHERE is_deleted = FALSE").await?;
        let members    = count("SELECT COUNT(*) FROM forum.user_profiles").await?;
        let reactions  = count("SELECT COUNT(*) FROM forum.reactions").await?;
        let online = db
            .fetch_scalar::<i64>(
                "SELECT COUNT(*) FROM forum.online WHERE last_seen_at > $1",
                params![Utc::now() - Duration::minutes(5)],
            )
            .await?;
        let latest_member = db
            .fetch_optional_scalar::<Uuid>(
                "SELECT user_id FROM forum.user_profiles ORDER BY created_at DESC LIMIT 1",
                params![],
            )
            .await?;
        Ok(ForumStats { categories, forums, topics, posts, members, reactions, online, latest_member })
    }

    /// Latest active members (by first forum activity).
    // Kept for the home "latest members / top contributors" widgets (V8); the
    // paginated members directory now covers the full list.
    #[allow(dead_code)]
    pub async fn latest_members(limit: i64, db: &DbPool) -> Result<Vec<Uuid>> {
        let ids: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT user_id FROM forum.user_profiles ORDER BY created_at DESC LIMIT $1",
                params![limit.clamp(1, 50)],
            )
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();
        Ok(ids)
    }

    /// Top contributors by post count.
    pub async fn top_posters(limit: i64, db: &DbPool) -> Result<Vec<(Uuid, i32)>> {
        let rows = db
            .fetch_all_as::<(Uuid, i32)>(
                "SELECT user_id, post_count FROM forum.user_profiles WHERE post_count > 0 \
                 ORDER BY post_count DESC LIMIT $1",
                params![limit.clamp(1, 50)],
            )
            .await?;
        Ok(rows)
    }
}
