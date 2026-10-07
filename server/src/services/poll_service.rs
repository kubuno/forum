use kubuno_db::dialect::SqlType;
use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::poll::{Poll, PollOptionResult, PollResults},
};

pub struct PollService;

impl PollService {
    /// Loads a poll by id (used to resolve its topic before a permission check).
    pub async fn find(poll_id: Uuid, db: &DbPool) -> Result<Poll> {
        db.fetch_optional_as::<Poll>(
            "SELECT * FROM forum.polls WHERE id = $1",
            params![poll_id],
        )
        .await?
        .ok_or_else(|| ForumError::NotFound(format!("Poll {poll_id}")))
    }

    /// Returns a topic's poll with per-option counts for the requesting user.
    pub async fn results(topic_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Option<PollResults>> {
        let poll = db
            .fetch_optional_as::<Poll>(
                "SELECT * FROM forum.polls WHERE topic_id = $1",
                params![topic_id],
            )
            .await?;
        let Some(poll) = poll else { return Ok(None) };
        Ok(Some(Self::build_results(poll, user_id, db).await?))
    }

    async fn build_results(poll: Poll, user_id: Uuid, db: &DbPool) -> Result<PollResults> {
        let options = db
            .fetch_all_as::<(Uuid, String)>(
                "SELECT id, text FROM forum.poll_options WHERE poll_id = $1 ORDER BY position, id",
                params![poll.id],
            )
            .await?;
        let counts = db
            .fetch_all_as::<(Uuid, i64)>(
                "SELECT option_id, COUNT(*) FROM forum.poll_votes WHERE poll_id = $1 GROUP BY option_id",
                params![poll.id],
            )
            .await?;
        let mine: Vec<Uuid> = db
            .fetch_all_as::<(Uuid,)>(
                "SELECT option_id FROM forum.poll_votes WHERE poll_id = $1 AND user_id = $2",
                params![poll.id, user_id],
            )
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();
        let total_voters: i64 = db
            .fetch_scalar(
                "SELECT COUNT(DISTINCT user_id) FROM forum.poll_votes WHERE poll_id = $1",
                params![poll.id],
            )
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
    pub async fn vote(poll_id: Uuid, user_id: Uuid, option_ids: &[Uuid], db: &DbPool) -> Result<PollResults> {
        let poll = db
            .fetch_optional_as::<Poll>(
                "SELECT * FROM forum.polls WHERE id = $1",
                params![poll_id],
            )
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

        let b = db.backend();
        // Only vote for an option that really belongs to this poll; a duplicate
        // (poll, option, user) row is a no-op.
        // A placeholder is never reused on the portable path, so `poll_id` and
        // `option_id` are bound again for the EXISTS guard ($4, $5).
        let insert_sql = format!(
            "INSERT {}INTO forum.poll_votes (poll_id, option_id, user_id) \
             SELECT $1, $2, $3 WHERE EXISTS (SELECT {} FROM forum.poll_options WHERE id = $4 AND poll_id = $5){}",
            b.insert_ignore_prefix(),
            b.cast("1", SqlType::BigInt),
            b.on_conflict_do_nothing(&["poll_id", "option_id", "user_id"]),
        );

        let mut tx = db.begin().await?;
        tx.execute(
            "DELETE FROM forum.poll_votes WHERE poll_id = $1 AND user_id = $2",
            params![poll_id, user_id],
        )
        .await?;
        for oid in &chosen {
            tx.execute(&insert_sql, params![poll_id, *oid, user_id, *oid, poll_id]).await?;
        }
        tx.commit().await?;

        Self::build_results(poll, user_id, db).await
    }
}
