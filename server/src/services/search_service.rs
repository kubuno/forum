//! Full-text search across post bodies and topic titles, made identical on the
//! three engines by `kubuno_db::search`: bodies and titles are reduced to
//! Snowball French stems and deaccented IN RUST at write time (stored in
//! `posts.body_norm` / `topics.title_norm`), and a query is put through the same
//! reduction and matched with a portable `LIKE`. This replaces the old
//! PostgreSQL-only `tsvector` / `websearch_to_tsquery` / `ts_rank_cd` /
//! `ts_headline` pipeline.
//!
//! Ranking mirrors the former `ts_rank_cd`: a hit in a topic title (weight A)
//! outranks a hit in a post body (weight B). The snippet, previously
//! `ts_headline`, is now built in Rust from the raw body around the first
//! matching query word, keeping the `\x01`/`\x02` highlight markers the
//! frontend expects.
//!
//! Reservation: `pg_trgm`'s typo tolerance is gone — a `LIKE '%stem%'` needs the
//! stem to appear as a substring. Stemming still folds inflections and the
//! normalizer folds accents, so inflected and accented queries still match.

use chrono::{DateTime, Duration, Utc};
use kubuno_db::search::{like_pattern, normalize, Weight};
use kubuno_db::{DbPool, DbQueryBuilder};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    services::permission_service::PermissionService,
};

const SEL_START: char = '\u{1}';
const SEL_STOP: char = '\u{2}';

#[derive(Debug, Serialize)]
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

/// Internal row: the visible columns plus the raw body, so the snippet can be
/// built in Rust for the returned rows.
#[derive(Debug, sqlx::FromRow)]
struct HitRow {
    post_id:     Uuid,
    topic_id:    Uuid,
    forum_id:    Uuid,
    author_id:   Uuid,
    topic_title: String,
    topic_slug:  String,
    body_md:     String,
    created_at:  DateTime<Utc>,
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

    /// The normalized columns that participate, with their weight class.
    fn fields(self) -> Vec<(&'static str, Weight)> {
        match self {
            SearchScope::Title => vec![("t.title_norm", Weight::A)],
            SearchScope::Body => vec![("p.body_norm", Weight::B)],
            SearchScope::All => vec![("t.title_norm", Weight::A), ("p.body_norm", Weight::B)],
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
    fn validate(query: &str) -> Result<()> {
        if query.chars().count() < 2 {
            return Err(ForumError::Validation(
                "la recherche doit contenir au moins 2 caractères".to_string(),
            ));
        }
        Ok(())
    }

    /// The distinct query stems, in order (empty when the query is all stopwords).
    fn terms(query: &str) -> Vec<String> {
        let mut terms: Vec<String> = Vec::new();
        for stem in normalize(query).split_whitespace() {
            if !stem.is_empty() && !terms.iter().any(|t| t == stem) {
                terms.push(stem.to_owned());
            }
        }
        terms
    }

    /// Appends the `WHERE` predicate shared by `search` and `count`. Keeping this
    /// in one place is what guarantees `count` can never be less restrictive than
    /// `search` (SEC-01): the same soft-delete/approval exclusion, the same
    /// `PermissionService::push_visible_forum` visibility filter, the same term
    /// match and the same advanced filters apply to both.
    fn push_where(
        qb: &mut DbQueryBuilder,
        user: &ForumUser,
        filters: &SearchFilters,
        terms: &[String],
    ) {
        qb.push("p.is_deleted = FALSE AND t.is_deleted = FALSE AND (p.is_approved = TRUE OR p.author_id = ");
        qb.push_bind(user.id);
        qb.push(") AND ");
        PermissionService::push_visible_forum(qb, "p.forum_id", user);

        // Every term must appear in at least one active field (AND over terms,
        // OR over fields), matched as a normalized `%stem%` substring.
        let fields = filters.scope.fields();
        for term in terms {
            qb.push(" AND (");
            for (i, (col, _)) in fields.iter().enumerate() {
                if i > 0 {
                    qb.push(" OR ");
                }
                qb.push(*col).push(" LIKE ").push_bind(like_pattern(term));
            }
            qb.push(")");
        }

        if let Some(author_id) = filters.author_id {
            qb.push(" AND p.author_id = ").push_bind(author_id);
        }
        if let Some(forum_ids) = &filters.forum_ids {
            if !forum_ids.is_empty() {
                qb.push(" AND p.forum_id").push_in(forum_ids.iter().copied());
            }
        }
        if let Some(days) = filters.days {
            if days > 0 {
                let cutoff = Utc::now() - Duration::days(days as i64);
                qb.push(" AND p.created_at >= ").push_bind(cutoff);
            }
        }
    }

    /// Full-text search across post bodies and topic titles.
    pub async fn search(
        user: &ForumUser,
        query: &str,
        filters: &SearchFilters,
        limit: i64,
        offset: i64,
        db: &DbPool,
    ) -> Result<Vec<SearchHit>> {
        Self::validate(query)?;
        let terms = Self::terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT p.id AS post_id, p.topic_id, p.forum_id, p.author_id, \
                    t.title AS topic_title, t.slug AS topic_slug, \
                    p.body_md, p.created_at \
               FROM forum.posts p \
               JOIN forum.topics t ON t.id = p.topic_id \
              WHERE ",
        );
        Self::push_where(&mut qb, user, filters, &terms);

        match filters.sort {
            SearchSort::Recent => {
                qb.push(" ORDER BY p.created_at DESC");
            }
            SearchSort::Relevance => {
                // A score summing each field's weight for every term it contains,
                // the portable stand-in for `ts_rank_cd`.
                qb.push(" ORDER BY (");
                let fields = filters.scope.fields();
                let mut first = true;
                for term in &terms {
                    for (col, weight) in &fields {
                        if !first {
                            qb.push(" + ");
                        }
                        first = false;
                        qb.push("(CASE WHEN ")
                            .push(*col)
                            .push(" LIKE ")
                            .push_bind(like_pattern(term))
                            .push(format!(" THEN {} ELSE 0 END)", weight.score()));
                    }
                }
                qb.push(") DESC, p.created_at DESC");
            }
        }
        qb.push_limit_offset(limit, offset);

        let rows: Vec<HitRow> = qb.fetch_all_as(db).await?;
        Ok(rows
            .into_iter()
            .map(|r| SearchHit {
                snippet: snippet(&r.body_md, query),
                post_id: r.post_id,
                topic_id: r.topic_id,
                forum_id: r.forum_id,
                author_id: r.author_id,
                topic_title: r.topic_title,
                topic_slug: r.topic_slug,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Total hits for the same query and filters, under the exact same visibility
    /// rules as `search` (SEC-01).
    pub async fn count(
        user: &ForumUser,
        query: &str,
        filters: &SearchFilters,
        db: &DbPool,
    ) -> Result<i64> {
        Self::validate(query)?;
        let terms = Self::terms(query);
        if terms.is_empty() {
            return Ok(0);
        }

        let mut qb = DbQueryBuilder::new(
            db.backend(),
            "SELECT COUNT(*) \
               FROM forum.posts p \
               JOIN forum.topics t ON t.id = p.topic_id \
              WHERE ",
        );
        Self::push_where(&mut qb, user, filters, &terms);
        let n: i64 = qb.fetch_scalar(db).await?;
        Ok(n)
    }
}

/// Build a highlighted excerpt of a post body around the first occurrence of a
/// query word, wrapped in the `\x01`/`\x02` markers the frontend renders. Falls
/// back to a plain leading excerpt when nothing matches (e.g. a stem-only hit).
fn snippet(body: &str, query: &str) -> String {
    const WINDOW: usize = 240;
    let lower = body.to_lowercase();
    let word = query
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .find(|w| w.chars().count() >= 2);

    if let Some(w) = word {
        if let Some(pos) = lower.find(&w) {
            // Center the window on the match, on char boundaries.
            let start = body[..pos]
                .char_indices()
                .rev()
                .nth(WINDOW / 4)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let match_end = pos + w.len();
            let end = body[match_end..]
                .char_indices()
                .nth(WINDOW / 2)
                .map(|(i, _)| match_end + i)
                .unwrap_or(body.len());
            let mut out = String::new();
            if start > 0 {
                out.push('…');
            }
            out.push_str(&body[start..pos]);
            out.push(SEL_START);
            out.push_str(&body[pos..match_end]);
            out.push(SEL_STOP);
            out.push_str(&body[match_end..end]);
            if end < body.len() {
                out.push('…');
            }
            return out;
        }
    }

    // No literal match: a plain leading excerpt.
    let end = body
        .char_indices()
        .nth(WINDOW)
        .map(|(i, _)| i)
        .unwrap_or(body.len());
    let mut out = body[..end].to_string();
    if end < body.len() {
        out.push('…');
    }
    out
}
