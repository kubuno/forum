use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::Result;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SearchHit {
    pub post_id:     Uuid,
    pub topic_id:    Uuid,
    pub forum_id:    Uuid,
    pub author_id:   Uuid,
    pub topic_title: String,
    pub topic_slug:  String,
    pub snippet:     String,
    pub created_at:  DateTime<Utc>,
}

pub struct SearchService;

impl SearchService {
    /// Case-insensitive substring search across post bodies and topic titles.
    /// NOTE: accent-insensitive ranking (unaccent / pg_trgm) is a later refinement.
    pub async fn search(query: &str, limit: i64, offset: i64, db: &PgPool) -> Result<Vec<SearchHit>> {
        // Escape LIKE wildcards in the user input.
        let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let rows = sqlx::query_as::<_, SearchHit>(
            "SELECT p.id AS post_id, p.topic_id, p.forum_id, p.author_id,
                    t.title AS topic_title, t.slug AS topic_slug,
                    LEFT(p.body_md, 240) AS snippet, p.created_at
               FROM forum.posts p
               JOIN forum.topics t ON t.id = p.topic_id
              WHERE p.body_md ILIKE $1 ESCAPE '\\' OR t.title ILIKE $1 ESCAPE '\\'
              ORDER BY p.created_at DESC
              LIMIT $2 OFFSET $3",
        )
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
