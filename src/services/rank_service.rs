use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::rank::{CreateRankDto, Rank, UpdateProfileDto, UpdateRankDto, UserProfile},
};

pub struct RankService;

impl RankService {
    // ── Ranks CRUD (admin only) ────────────────────────────────────────────────

    pub async fn list(db: &PgPool) -> Result<Vec<Rank>> {
        let rows = sqlx::query_as::<_, Rank>(
            "SELECT * FROM forum.ranks ORDER BY is_special, min_posts",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(dto: CreateRankDto, db: &PgPool) -> Result<Rank> {
        let row = sqlx::query_as::<_, Rank>(
            "INSERT INTO forum.ranks (title, min_posts, is_special, badge)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(&dto.title)
        .bind(dto.min_posts)
        .bind(dto.is_special)
        .bind(&dto.badge)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(id: Uuid, dto: UpdateRankDto, db: &PgPool) -> Result<Rank> {
        let row = sqlx::query_as::<_, Rank>(
            "UPDATE forum.ranks SET
                title      = COALESCE($2, title),
                min_posts  = COALESCE($3, min_posts),
                is_special = COALESCE($4, is_special),
                badge      = COALESCE($5, badge)
             WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&dto.title)
        .bind(dto.min_posts)
        .bind(dto.is_special)
        .bind(&dto.badge)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Rank {id}")))?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, db: &PgPool) -> Result<()> {
        let res = sqlx::query("DELETE FROM forum.ranks WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        if res.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Rank {id}")));
        }
        Ok(())
    }

    // ── User profiles ───────────────────────────────────────────────────────────

    pub async fn get_profile(user_id: Uuid, db: &PgPool) -> Result<UserProfile> {
        if let Some(p) = sqlx::query_as::<_, UserProfile>(
            "SELECT * FROM forum.user_profiles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?
        {
            return Ok(p);
        }
        // Create a default profile on first access.
        let p = sqlx::query_as::<_, UserProfile>(
            "INSERT INTO forum.user_profiles (user_id) VALUES ($1)
             ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
             RETURNING *",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(p)
    }

    pub async fn update_signature(user_id: Uuid, dto: UpdateProfileDto, db: &PgPool) -> Result<UserProfile> {
        Self::get_profile(user_id, db).await?; // ensure the row exists
        let p = sqlx::query_as::<_, UserProfile>(
            "UPDATE forum.user_profiles SET
                signature_md = COALESCE($2, signature_md),
                bio_md       = COALESCE($3, bio_md),
                location     = COALESCE($4, location),
                website      = COALESCE($5, website),
                custom_title = COALESCE($6, custom_title)
             WHERE user_id = $1 RETURNING *",
        )
        .bind(user_id)
        .bind(&dto.signature_md)
        .bind(&dto.bio_md)
        .bind(&dto.location)
        .bind(&dto.website)
        .bind(&dto.custom_title)
        .fetch_one(db)
        .await?;
        Ok(p)
    }

    /// Bump a user's topic counter (called when they open a new topic).
    pub async fn bump_topic_count(conn: &mut PgConnection, user_id: Uuid, delta: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO forum.user_profiles (user_id, topic_count)
             VALUES ($1, GREATEST($2, 0))
             ON CONFLICT (user_id) DO UPDATE
                SET topic_count = GREATEST(forum.user_profiles.topic_count + $2, 0)",
        )
        .bind(user_id)
        .bind(delta)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Recent posts authored by a user (their public activity feed).
    pub async fn activity(user_id: Uuid, limit: i64, db: &PgPool) -> Result<Vec<crate::models::post::Post>> {
        let rows = sqlx::query_as::<_, crate::models::post::Post>(
            "SELECT * FROM forum.posts WHERE author_id = $1 AND is_deleted = FALSE
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(limit.clamp(1, 50))
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Bump a user's post counter (in a transaction) and refresh their rank.
    pub async fn bump_post_count(conn: &mut PgConnection, user_id: Uuid, delta: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO forum.user_profiles (user_id, post_count)
             VALUES ($1, GREATEST($2, 0))
             ON CONFLICT (user_id) DO UPDATE
                SET post_count = GREATEST(forum.user_profiles.post_count + $2, 0)",
        )
        .bind(user_id)
        .bind(delta)
        .execute(&mut *conn)
        .await?;

        // Refresh the (non-special) rank matching the new post count.
        sqlx::query(
            "UPDATE forum.user_profiles p
                SET rank_id = (
                    SELECT r.id FROM forum.ranks r
                     WHERE r.is_special = FALSE AND r.min_posts <= p.post_count
                     ORDER BY r.min_posts DESC LIMIT 1
                )
              WHERE p.user_id = $1
                AND (p.rank_id IS NULL OR p.rank_id NOT IN (SELECT id FROM forum.ranks WHERE is_special = TRUE))",
        )
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
