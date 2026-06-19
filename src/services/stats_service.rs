use serde::Serialize;
use sqlx::PgPool;
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
    pub async fn global(db: &PgPool) -> Result<ForumStats> {
        let one = |sql: &'static str| async move {
            sqlx::query_scalar::<_, i64>(sql).fetch_one(db).await
        };
        let categories = one("SELECT COUNT(*) FROM forum.categories").await?;
        let forums     = one("SELECT COUNT(*) FROM forum.forums").await?;
        let topics     = one("SELECT COUNT(*) FROM forum.topics").await?;
        let posts      = one("SELECT COUNT(*) FROM forum.posts WHERE is_deleted = FALSE").await?;
        let members    = one("SELECT COUNT(*) FROM forum.user_profiles").await?;
        let reactions  = one("SELECT COUNT(*) FROM forum.reactions").await?;
        let online     = one("SELECT COUNT(*) FROM forum.online WHERE last_seen_at > NOW() - INTERVAL '5 minutes'").await?;
        let latest_member = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM forum.user_profiles ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(db)
        .await?;
        Ok(ForumStats { categories, forums, topics, posts, members, reactions, online, latest_member })
    }

    /// Latest active members (by first forum activity).
    pub async fn latest_members(limit: i64, db: &PgPool) -> Result<Vec<Uuid>> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM forum.user_profiles ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 50))
        .fetch_all(db)
        .await?;
        Ok(ids)
    }

    /// Top contributors by post count.
    pub async fn top_posters(limit: i64, db: &PgPool) -> Result<Vec<(Uuid, i32)>> {
        let rows = sqlx::query_as::<_, (Uuid, i32)>(
            "SELECT user_id, post_count FROM forum.user_profiles WHERE post_count > 0
             ORDER BY post_count DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 50))
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
