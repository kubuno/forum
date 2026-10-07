use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{errors::Result, models::topic::Topic};

pub struct BookmarkService;

impl BookmarkService {
    /// Toggles a bookmark on a topic. Returns whether it is now bookmarked.
    pub async fn toggle(user_id: Uuid, topic_id: Uuid, db: &DbPool) -> Result<bool> {
        let existed: Option<(Uuid,)> = db
            .fetch_optional_as::<(Uuid,)>(
                "SELECT user_id FROM forum.bookmarks WHERE user_id = $1 AND topic_id = $2",
                params![user_id, topic_id],
            )
            .await?;
        if existed.is_some() {
            db.execute(
                "DELETE FROM forum.bookmarks WHERE user_id = $1 AND topic_id = $2",
                params![user_id, topic_id],
            )
            .await?;
            Ok(false)
        } else {
            let b = db.backend();
            let sql = format!(
                "INSERT {}INTO forum.bookmarks (user_id, topic_id) VALUES ($1, $2){}",
                b.insert_ignore_prefix(),
                b.on_conflict_do_nothing(&["user_id", "topic_id"]),
            );
            db.execute(&sql, params![user_id, topic_id]).await?;
            Ok(true)
        }
    }

    pub async fn is_bookmarked(user_id: Uuid, topic_id: Uuid, db: &DbPool) -> Result<bool> {
        let n: i64 = db
            .fetch_scalar(
                "SELECT COUNT(*) FROM forum.bookmarks WHERE user_id = $1 AND topic_id = $2",
                params![user_id, topic_id],
            )
            .await?;
        Ok(n > 0)
    }

    pub async fn list(user_id: Uuid, db: &DbPool) -> Result<Vec<Topic>> {
        let topics = db
            .fetch_all_as::<Topic>(
                "SELECT t.* FROM forum.topics t \
                 JOIN forum.bookmarks b ON b.topic_id = t.id \
                 WHERE b.user_id = $1 \
                 ORDER BY b.created_at DESC",
                params![user_id],
            )
            .await?;
        Ok(topics)
    }
}
