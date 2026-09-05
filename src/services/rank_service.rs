use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::rank::{BriefProfile, CreateRankDto, MemberRow, Rank, UpdateProfileDto, UpdateRankDto, UserProfile},
};

pub struct RankService;

impl RankService {
    /// A page of the members directory, ordered by post count (default), join
    /// date or last activity. `sort` comes from a fixed set, never raw input, so
    /// it is safe to inline into the ORDER BY.
    pub async fn members_page(
        sort: &str,
        limit: i64,
        offset: i64,
        db: &PgPool,
    ) -> Result<(Vec<MemberRow>, i64)> {
        let order = match sort {
            "recent" => "p.created_at DESC",
            "active" => "p.last_seen_at DESC NULLS LAST, p.post_count DESC",
            _ => "p.post_count DESC, p.created_at DESC",
        };
        let rows = sqlx::query_as::<_, MemberRow>(&format!(
            "SELECT p.user_id, p.post_count, r.title AS rank_title, r.badge AS rank_badge, \
                    p.last_seen_at, p.created_at \
               FROM forum.user_profiles p \
               LEFT JOIN forum.ranks r ON r.id = p.rank_id \
              ORDER BY {order} LIMIT $1 OFFSET $2"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM forum.user_profiles")
            .fetch_one(db)
            .await?;
        Ok((rows, total))
    }
    /// Compact profiles for a batch of users, joined to their rank, for the post
    /// listing's author column. Missing users simply have no row (a member who
    /// has never posted has no profile yet). Signatures are stripped unless they
    /// are enabled instance-wide.
    pub async fn brief_profiles(
        uids: &[Uuid],
        include_signatures: bool,
        db: &PgPool,
    ) -> Result<Vec<BriefProfile>> {
        let mut rows = sqlx::query_as::<_, BriefProfile>(
            "SELECT p.user_id, p.post_count, p.custom_title,
                    r.title AS rank_title, r.badge AS rank_badge, p.signature_md
               FROM forum.user_profiles p
               LEFT JOIN forum.ranks r ON r.id = p.rank_id
              WHERE p.user_id = ANY($1)",
        )
        .bind(uids)
        .fetch_all(db)
        .await?;
        if !include_signatures {
            for row in &mut rows {
                row.signature_md = None;
            }
        }
        Ok(rows)
    }

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

    /// Assigns (or clears, with `rank_id = None`) a user's SPECIAL rank —
    /// admin only, enforced by the handler. Only a rank flagged `is_special`
    /// may be assigned this way: the non-special ranks are earned
    /// automatically from post count in `bump_post_count` and stay off-limits
    /// here to keep that ladder meaningful. Clearing never touches the
    /// automatic rank — the next post simply reassigns it, same as today when
    /// `rank_id` is null.
    pub async fn assign_special(user_id: Uuid, rank_id: Option<Uuid>, db: &PgPool) -> Result<UserProfile> {
        if let Some(rid) = rank_id {
            let is_special: Option<bool> =
                sqlx::query_scalar("SELECT is_special FROM forum.ranks WHERE id = $1")
                    .bind(rid)
                    .fetch_optional(db)
                    .await?;
            match is_special {
                Some(true) => {}
                Some(false) => return Err(ForumError::Validation("this rank is not a special rank".into())),
                None => return Err(ForumError::NotFound(format!("Rank {rid}"))),
            }
        }
        Self::get_profile(user_id, db).await?; // ensure the row exists
        let p = sqlx::query_as::<_, UserProfile>(
            "UPDATE forum.user_profiles SET rank_id = $2 WHERE user_id = $1 RETURNING *",
        )
        .bind(user_id)
        .bind(rank_id)
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

    /// Recent posts authored by a user (their public activity feed). Approved
    /// only: this page is read by everyone, so it must not be where a message
    /// still waiting for a moderator becomes visible.
    pub async fn activity(user_id: Uuid, limit: i64, db: &PgPool) -> Result<Vec<crate::models::post::Post>> {
        let rows = sqlx::query_as::<_, crate::models::post::Post>(
            "SELECT * FROM forum.posts
              WHERE author_id = $1 AND is_deleted = FALSE AND is_approved
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
