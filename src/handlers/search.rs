use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    errors::Result,
    handlers::Pagination,
    services::search_service::SearchService,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q:      Option<String>,
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

pub async fn search(State(state): State<AppState>, Query(q): Query<SearchQuery>) -> Result<Json<Value>> {
    let term = q.q.unwrap_or_default();
    let term = term.trim();
    if term.is_empty() {
        return Ok(Json(json!({ "results": [] })));
    }
    let (limit, offset) = Pagination { limit: q.limit, offset: q.offset }.resolve(20, 100);
    let results = SearchService::search(term, limit, offset, &state.db).await?;
    Ok(Json(json!({ "results": results })))
}
