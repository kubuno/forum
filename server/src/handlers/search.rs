use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    handlers::Pagination,
    middleware::ForumUser,
    services::search_service::{SearchFilters, SearchScope, SearchService, SearchSort},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q:         Option<String>,
    pub limit:     Option<i64>,
    pub offset:    Option<i64>,
    /// Restrict to posts authored by this user.
    pub author_id: Option<Uuid>,
    /// Comma-separated list of forum ids to restrict to.
    pub forum_ids: Option<String>,
    /// "all" (default) | "title" | "body".
    pub scope:     Option<String>,
    /// "relevance" (default) | "recent".
    pub sort:      Option<String>,
    /// Restrict to posts created within the last N days.
    pub days:      Option<i32>,
}

pub async fn search(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>> {
    let term = q.q.unwrap_or_default();
    let term = term.trim();
    // A one-character query forces a full-text lookup for near-zero signal;
    // require a couple of characters before touching the database.
    if term.chars().count() < 2 {
        return Ok(Json(json!({ "results": [], "total": 0 })));
    }
    let (limit, offset) = Pagination { limit: q.limit, offset: q.offset }.resolve(20, 100);

    let forum_ids = q.forum_ids.as_deref().map(parse_uuid_csv).transpose()?;
    let filters = SearchFilters {
        author_id: q.author_id,
        forum_ids,
        scope: SearchScope::parse(q.scope.as_deref()),
        sort:  SearchSort::parse(q.sort.as_deref()),
        days:  q.days,
    };

    let results = SearchService::search(&user, term, &filters, limit, offset, &state.db).await?;
    let total = SearchService::count(&user, term, &filters, &state.db).await?;
    Ok(Json(json!({ "results": results, "total": total })))
}

/// Parses a comma-separated list of forum UUIDs, ignoring blank entries so a
/// trailing comma or an empty string does not error.
fn parse_uuid_csv(raw: &str) -> Result<Vec<Uuid>> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<Uuid>()
                .map_err(|_| ForumError::Validation(format!("identifiant de forum invalide : {s}")))
        })
        .collect()
}
