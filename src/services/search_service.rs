use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    services::permission_service::PermissionService,
};

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

/// Which indexed column(s) the query must match against.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    #[default]
    All,
    Title,
    Body,
}

impl SearchScope {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("title") => Self::Title,
            Some("body") => Self::Body,
            _ => Self::All,
        }
    }
}

/// How hits are ordered.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SearchSort {
    #[default]
    Relevance,
    Recent,
}

impl SearchSort {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("recent") => Self::Recent,
            _ => Self::Relevance,
        }
    }
}

/// Advanced filters layered on top of the free-text query. Every field is
/// optional and defaults to "no restriction" — the plain search bar leaves
/// this at `SearchFilters::default()`, and the advanced panel narrows it.
/// Both UIs feed the exact same `search`/`count` call with the same object,
/// so they can never drift out of sync with each other.
#[derive(Debug, Default, Clone)]
pub struct SearchFilters {
    pub author_id: Option<Uuid>,
    pub forum_ids: Option<Vec<Uuid>>,
    pub scope:     SearchScope,
    pub sort:      SearchSort,
    /// Restrict to posts created within the last N days.
    pub days:      Option<i32>,
}

pub struct SearchService;

impl SearchService {
    /// A one- or zero-character query forces `websearch_to_tsquery` to do
    /// near-zero work for near-zero signal, and an empty search vector match
    /// pattern is a cheap way to accidentally full-scan; require a couple of
    /// characters before touching the database at all.
    fn validate(query: &str) -> Result<()> {
        if query.chars().count() < 2 {
            return Err(ForumError::Validation(
                "la recherche doit contenir au moins 2 caractères".to_string(),
            ));
        }
        Ok(())
    }

    /// Appends `WITH q AS (SELECT websearch_to_tsquery(...) AS tsq) `, ahead
    /// of the caller's own `SELECT ... FROM ...`. `websearch_to_tsquery`
    /// natively understands quoted phrases, `OR`, and `-exclusion`, which is
    /// exactly the query syntax a search bar wants to expose to users.
    fn push_cte(qb: &mut QueryBuilder<'_, Postgres>, query: &str) {
        qb.push("WITH q AS (SELECT websearch_to_tsquery('french', ")
            .push_bind(query.to_string())
            .push(") AS tsq) ");
    }

    /// Appends the `WHERE` predicate shared by `search` and `count`. Keeping
    /// this in one place is what guarantees `count` can never be less
    /// restrictive than `search` (SEC-01): the same soft-delete/approval
    /// exclusion, the same `PermissionService::push_visible_forum` visibility
    /// filter, and the same advanced filters apply to both, however the
    /// feature grows later.
    fn push_where(qb: &mut QueryBuilder<'_, Postgres>, user: &ForumUser, filters: &SearchFilters) {
        qb.push("p.is_deleted = FALSE AND t.is_deleted = FALSE AND (p.is_approved = TRUE OR p.author_id = ");
        qb.push_bind(user.id);
        qb.push(") AND ");
        PermissionService::push_visible_forum(qb, "p.forum_id", user);
        qb.push(" AND ");
        match filters.scope {
            SearchScope::All => qb.push("(p.search_vector @@ q.tsq OR t.search_vector @@ q.tsq)"),
            SearchScope::Title => qb.push("t.search_vector @@ q.tsq"),
            SearchScope::Body => qb.push("p.search_vector @@ q.tsq"),
        };

        if let Some(author_id) = filters.author_id {
            qb.push(" AND p.author_id = ").push_bind(author_id);
        }
        if let Some(forum_ids) = &filters.forum_ids {
            if !forum_ids.is_empty() {
                qb.push(" AND p.forum_id = ANY(")
                    .push_bind(forum_ids.clone())
                    .push("::uuid[])");
            }
        }
        if let Some(days) = filters.days {
            if days > 0 {
                qb.push(" AND p.created_at >= NOW() - make_interval(days => ")
                    .push_bind(days)
                    .push(")");
            }
        }
    }

    /// Full-text search across post bodies and topic titles.
    ///
    /// Results are restricted to forums the caller may view and never leak
    /// posts that are unapproved (unless authored by the caller) or
    /// soft-deleted — the same visibility rules the topic listing enforces
    /// (SEC-01).
    pub async fn search(
        user: &ForumUser,
        query: &str,
        filters: &SearchFilters,
        limit: i64,
        offset: i64,
        db: &PgPool,
    ) -> Result<Vec<SearchHit>> {
        Self::validate(query)?;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("");
        Self::push_cte(&mut qb, query);
        qb.push(
            "SELECT p.id AS post_id, p.topic_id, p.forum_id, p.author_id, \
                    t.title AS topic_title, t.slug AS topic_slug, \
                    ts_headline('french', p.body_md, q.tsq, \
                        E'MaxWords=30,MinWords=15,StartSel=\\x01,StopSel=\\x02') AS snippet, \
                    p.created_at \
               FROM forum.posts p \
               JOIN forum.topics t ON t.id = p.topic_id \
               CROSS JOIN q \
              WHERE ",
        );
        Self::push_where(&mut qb, user, filters);

        match filters.sort {
            SearchSort::Recent => {
                qb.push(" ORDER BY p.created_at DESC");
            }
            SearchSort::Relevance => {
                qb.push(
                    " ORDER BY (ts_rank_cd(p.search_vector, q.tsq) + ts_rank_cd(t.search_vector, q.tsq)) DESC, \
                        p.created_at DESC",
                );
            }
        }
        qb.push(" LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);

        let rows = qb.build_query_as::<SearchHit>().fetch_all(db).await?;
        Ok(rows)
    }

    /// Total hits for the same query and filters, under the exact same
    /// visibility rules as `search` (SEC-01) — the count must never leak the
    /// existence of content the caller could not otherwise see.
    pub async fn count(
        user: &ForumUser,
        query: &str,
        filters: &SearchFilters,
        db: &PgPool,
    ) -> Result<i64> {
        Self::validate(query)?;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("");
        Self::push_cte(&mut qb, query);
        qb.push(
            "SELECT COUNT(*) \
               FROM forum.posts p \
               JOIN forum.topics t ON t.id = p.topic_id \
               CROSS JOIN q \
              WHERE ",
        );
        Self::push_where(&mut qb, user, filters);

        let n: i64 = qb.build_query_scalar().fetch_one(db).await?;
        Ok(n)
    }
}
