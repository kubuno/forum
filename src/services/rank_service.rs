use kubuno_db::{new_id, params, DbPool, DbQueryBuilder, DbTx};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::rank::{BriefProfile, CreateRankDto, MemberRow, Rank, UpdateProfileDto, UpdateRankDto, UserProfile},
};

pub struct RankService;

impl RankService {
    /// A page of the members directory, ordered by post count (default), join
    /// date or last activity. `sort` only selects between whole queries that are
    /// assembled from literals at compile time, so no request data ever reaches
    /// the SQL text.
    pub async fn members_page(
        sort: &str,
        limit: i64,
        offset: i64,
        db: &DbPool,
    ) -> Result<(Vec<MemberRow>, i64)> {
        macro_rules! members_sql {
            ($order:literal) => {
                concat!(
                    "SELECT p.user_id, p.post_count, r.title AS rank_title, r.badge AS rank_badge, \
                            p.last_seen_at, p.created_at \
                       FROM forum.user_profiles p \
                       LEFT JOIN forum.ranks r ON r.id = p.rank_id \
                      ORDER BY ",
                    $order,
                    " LIMIT $1 OFFSET $2"
                )
            };
        }
        // `(col IS NULL)` first is the portable stand-in for `DESC NULLS LAST`:
        // it sorts NULLs after every non-NULL on all three engines.
        let sql: &'static str = match sort {
            "recent" => members_sql!("p.created_at DESC"),
            "active" => members_sql!("(p.last_seen_at IS NULL), p.last_seen_at DESC, p.post_count DESC"),
            _ => members_sql!("p.post_count DESC, p.created_at DESC"),
        };
        let rows = db.fetch_all_as::<MemberRow>(sql, params![limit, offset]).await?;
        let total: i64 = db
            .fetch_scalar("SELECT COUNT(*) FROM forum.user_profiles", params![])
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
        db: &DbPool,
    ) -> Result<Vec<BriefProfile>> {
        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT p.user_id, p.post_count, p.custom_title, \
                    r.title AS rank_title, r.badge AS rank_badge, p.signature_md \
               FROM forum.user_profiles p \
               LEFT JOIN forum.ranks r ON r.id = p.rank_id \
              WHERE p.user_id",
        );
        qb.push_in(uids.iter().copied());
        let mut rows = qb.fetch_all_as::<BriefProfile>(db).await?;
        if !include_signatures {
            for row in &mut rows {
                row.signature_md = None;
            }
        }
        Ok(rows)
    }

    // ── Ranks CRUD (admin only) ────────────────────────────────────────────────

    pub async fn list(db: &DbPool) -> Result<Vec<Rank>> {
        let rows = db
            .fetch_all_as::<Rank>(
                "SELECT * FROM forum.ranks ORDER BY is_special, min_posts",
                params![],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(dto: CreateRankDto, db: &DbPool) -> Result<Rank> {
        let id = new_id();
        db.execute(
            "INSERT INTO forum.ranks (id, title, min_posts, is_special, badge) \
             VALUES ($1, $2, $3, $4, $5)",
            params![id, dto.title, dto.min_posts, dto.is_special, dto.badge],
        )
        .await?;
        db.fetch_one_as::<Rank>("SELECT * FROM forum.ranks WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn update(id: Uuid, dto: UpdateRankDto, db: &DbPool) -> Result<Rank> {
        let affected = db
            .execute(
                "UPDATE forum.ranks SET \
                    title      = COALESCE($1, title), \
                    min_posts  = COALESCE($2, min_posts), \
                    is_special = COALESCE($3, is_special), \
                    badge      = COALESCE($4, badge) \
                 WHERE id = $5",
                params![dto.title, dto.min_posts, dto.is_special, dto.badge, id],
            )
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Rank {id}")));
        }
        db.fetch_one_as::<Rank>("SELECT * FROM forum.ranks WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, db: &DbPool) -> Result<()> {
        let affected = db
            .execute("DELETE FROM forum.ranks WHERE id = $1", params![id])
            .await?;
        if affected == 0 {
            return Err(ForumError::NotFound(format!("Rank {id}")));
        }
        Ok(())
    }

    // ── User profiles ───────────────────────────────────────────────────────────

    pub async fn get_profile(user_id: Uuid, db: &DbPool) -> Result<UserProfile> {
        if let Some(p) = db
            .fetch_optional_as::<UserProfile>(
                "SELECT * FROM forum.user_profiles WHERE user_id = $1",
                params![user_id],
            )
            .await?
        {
            return Ok(p);
        }
        // Create a default profile on first access (a no-op if a concurrent
        // request already created it).
        let b = db.backend();
        db.execute(
            &format!(
                "INSERT {}INTO forum.user_profiles (user_id) VALUES ($1){}",
                b.insert_ignore_prefix(),
                b.on_conflict_do_nothing(&["user_id"])
            ),
            params![user_id],
        )
        .await?;
        db.fetch_one_as::<UserProfile>(
            "SELECT * FROM forum.user_profiles WHERE user_id = $1",
            params![user_id],
        )
        .await
        .map_err(Into::into)
    }

    /// Assigns (or clears, with `rank_id = None`) a user's SPECIAL rank —
    /// admin only, enforced by the handler. Only a rank flagged `is_special`
    /// may be assigned this way: the non-special ranks are earned
    /// automatically from post count in `bump_post_count` and stay off-limits
    /// here to keep that ladder meaningful. Clearing never touches the
    /// automatic rank — the next post simply reassigns it, same as today when
    /// `rank_id` is null.
    pub async fn assign_special(user_id: Uuid, rank_id: Option<Uuid>, db: &DbPool) -> Result<UserProfile> {
        if let Some(rid) = rank_id {
            let is_special: Option<bool> = db
                .fetch_optional_scalar(
                    "SELECT is_special FROM forum.ranks WHERE id = $1",
                    params![rid],
                )
                .await?;
            match is_special {
                Some(true) => {}
                Some(false) => return Err(ForumError::Validation("this rank is not a special rank".into())),
                None => return Err(ForumError::NotFound(format!("Rank {rid}"))),
            }
        }
        Self::get_profile(user_id, db).await?; // ensure the row exists
        db.execute(
            "UPDATE forum.user_profiles SET rank_id = $1 WHERE user_id = $2",
            params![rank_id, user_id],
        )
        .await?;
        db.fetch_one_as::<UserProfile>(
            "SELECT * FROM forum.user_profiles WHERE user_id = $1",
            params![user_id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn update_signature(user_id: Uuid, dto: UpdateProfileDto, db: &DbPool) -> Result<UserProfile> {
        Self::get_profile(user_id, db).await?; // ensure the row exists
        db.execute(
            "UPDATE forum.user_profiles SET \
                signature_md = COALESCE($1, signature_md), \
                bio_md       = COALESCE($2, bio_md), \
                location     = COALESCE($3, location), \
                website      = COALESCE($4, website), \
                custom_title = COALESCE($5, custom_title) \
             WHERE user_id = $6",
            params![
                dto.signature_md,
                dto.bio_md,
                dto.location,
                dto.website,
                dto.custom_title,
                user_id
            ],
        )
        .await?;
        db.fetch_one_as::<UserProfile>(
            "SELECT * FROM forum.user_profiles WHERE user_id = $1",
            params![user_id],
        )
        .await
        .map_err(Into::into)
    }

    /// Bump a user's topic counter (called when they open a new topic). The row
    /// is ensured to exist first, then the delta is applied floored at zero — a
    /// portable read-modify-write instead of PostgreSQL's `GREATEST` upsert.
    pub async fn bump_topic_count(tx: &mut DbTx, user_id: Uuid, delta: i32) -> Result<()> {
        let b = tx.backend();
        tx.execute(
            &format!(
                "INSERT {}INTO forum.user_profiles (user_id) VALUES ($1){}",
                b.insert_ignore_prefix(),
                b.on_conflict_do_nothing(&["user_id"])
            ),
            params![user_id],
        )
        .await?;
        tx.execute(
            "UPDATE forum.user_profiles \
                SET topic_count = CASE WHEN topic_count + $1 > 0 THEN topic_count + $2 ELSE 0 END \
              WHERE user_id = $3",
            params![delta, delta, user_id],
        )
        .await?;
        Ok(())
    }

    /// Recent posts authored by a user (their public activity feed). Approved
    /// only: this page is read by everyone, so it must not be where a message
    /// still waiting for a moderator becomes visible.
    pub async fn activity(user_id: Uuid, limit: i64, db: &DbPool) -> Result<Vec<crate::models::post::Post>> {
        let rows = db
            .fetch_all_as::<crate::models::post::Post>(
                "SELECT * FROM forum.posts \
                  WHERE author_id = $1 AND is_deleted = FALSE AND is_approved \
                 ORDER BY created_at DESC LIMIT $2",
                params![user_id, limit.clamp(1, 50)],
            )
            .await?;
        Ok(rows)
    }

    /// Bump a user's post counter (in a transaction) and refresh their rank.
    pub async fn bump_post_count(tx: &mut DbTx, user_id: Uuid, delta: i32) -> Result<()> {
        let b = tx.backend();
        tx.execute(
            &format!(
                "INSERT {}INTO forum.user_profiles (user_id) VALUES ($1){}",
                b.insert_ignore_prefix(),
                b.on_conflict_do_nothing(&["user_id"])
            ),
            params![user_id],
        )
        .await?;
        tx.execute(
            "UPDATE forum.user_profiles \
                SET post_count = CASE WHEN post_count + $1 > 0 THEN post_count + $2 ELSE 0 END \
              WHERE user_id = $3",
            params![delta, delta, user_id],
        )
        .await?;

        // Refresh the (non-special) rank matching the new post count.
        tx.execute(
            "UPDATE forum.user_profiles p \
                SET rank_id = ( \
                    SELECT r.id FROM forum.ranks r \
                     WHERE r.is_special = FALSE AND r.min_posts <= p.post_count \
                     ORDER BY r.min_posts DESC LIMIT 1 \
                ) \
              WHERE p.user_id = $1 \
                AND (p.rank_id IS NULL OR p.rank_id NOT IN (SELECT id FROM forum.ranks WHERE is_special = TRUE))",
            params![user_id],
        )
        .await?;
        Ok(())
    }
}
