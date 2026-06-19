use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::poll::{Poll, PollOptionResult, PollResults},
};

pub struct PollService;

impl PollService {
    /// Returns a topic's poll with per-option counts for the requesting user.
    pub async fn results(topic_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Option<PollResults>> {
        let poll = sqlx::query_as::<_, Poll>("SELECT * FROM forum.polls WHERE topic_id = $1")
            .bind(topic_id)
            .fetch_optional(db)
            .await?;
        let Some(poll) = poll else { return Ok(None) };
        Ok(Some(Self::build_results(poll, user_id, db).await?))
    }

    async fn build_results(poll: Poll, user_id: Uuid, db: &PgPool) -> Result<PollResults> {
        let options = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, text FROM forum.poll_options WHERE poll_id = $1 ORDER BY position, id",
        )
        .bind(poll.id)
        .fetch_all(db)
        .await?;
        let counts = sqlx::query_as::<_, (Uuid, i64)>(
            "SELECT option_id, COUNT(*) FROM forum.poll_votes WHERE poll_id = $1 GROUP BY option_id",
        )
        .bind(poll.id)
        .fetch_all(db)
        .await?;
        let mine: Vec<Uuid> = sqlx::query_scalar(
            "SELECT option_id FROM forum.poll_votes WHERE poll_id = $1 AND user_id = $2",
        )
        .bind(poll.id)
        .bind(user_id)
        .fetch_all(db)
        .await?;
        let total_voters: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT user_id) FROM forum.poll_votes WHERE poll_id = $1",
        )
        .bind(poll.id)
        .fetch_one(db)
        .await?;

        let count_for = |id: Uuid| counts.iter().find(|(oid, _)| *oid == id).map(|(_, c)| *c).unwrap_or(0);
        let opt_results = options
            .into_iter()
            .map(|(id, text)| PollOptionResult { votes: count_for(id), me: mine.contains(&id), id, text })
            .collect();

        let is_closed = poll.closes_at.map(|c| c < chrono::Utc::now()).unwrap_or(false);
        Ok(PollResults {
            options: opt_results,
            total_voters,
            has_voted: !mine.is_empty(),
            is_closed,
            poll,
        })
    }

    /// Casts (or replaces) a user's vote(s).
    pub async fn vote(poll_id: Uuid, user_id: Uuid, option_ids: &[Uuid], db: &PgPool) -> Result<PollResults> {
        let poll = sqlx::query_as::<_, Poll>("SELECT * FROM forum.polls WHERE id = $1")
            .bind(poll_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ForumError::NotFound(format!("Poll {poll_id}")))?;

        if poll.closes_at.map(|c| c < chrono::Utc::now()).unwrap_or(false) {
            return Err(ForumError::Conflict("this poll is closed".into()));
        }
        if option_ids.is_empty() {
            return Err(ForumError::Validation("select at least one option".into()));
        }
        let chosen: Vec<Uuid> = if poll.is_multiple {
            option_ids.to_vec()
        } else {
            vec![option_ids[0]]
        };

        let mut tx = db.begin().await?;
        sqlx::query("DELETE FROM forum.poll_votes WHERE poll_id = $1 AND user_id = $2")
            .bind(poll_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        for oid in &chosen {
            sqlx::query(
                "INSERT INTO forum.poll_votes (poll_id, option_id, user_id)
                 SELECT $1, $2, $3 WHERE EXISTS (SELECT 1 FROM forum.poll_options WHERE id = $2 AND poll_id = $1)
                 ON CONFLICT DO NOTHING",
            )
            .bind(poll_id)
            .bind(oid)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        Self::build_results(poll, user_id, db).await
    }
}
