use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use axum::http::StatusCode;

use crate::{
    errors::Result,
    handlers::Pagination,
    middleware::ForumUser,
    services::{
        engagement_service::EngagementService, permission_service::PermissionService,
        presence_service::PresenceService, rank_service::RankService, stats_service::StatsService,
        topic_service::TopicService,
    },
    state::AppState,
};

#[derive(Deserialize)]
pub struct HeartbeatDto {
    pub path: Option<String>,
}

/// POST /me/heartbeat — keeps the user listed as online.
pub async fn heartbeat(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
    Json(dto): Json<HeartbeatDto>,
) -> Result<Json<Value>> {
    PresenceService::heartbeat(user.id, dto.path.as_deref(), &state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

/// POST /me/read-all — marks every visible, non-deleted topic across every
/// forum the caller can see as read, up to its latest message.
pub async fn read_all(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<StatusCode> {
    EngagementService::mark_all_read(&user, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /online — user ids active in the last 5 minutes.
pub async fn online(State(state): State<AppState>) -> Result<Json<Value>> {
    let ids = PresenceService::who_online(5, &state.db).await?;
    Ok(Json(json!({ "user_ids": ids })))
}

/// GET /stats — community-wide totals.
pub async fn stats(State(state): State<AppState>) -> Result<Json<Value>> {
    let stats = StatsService::global(&state.db).await?;
    Ok(Json(json!({ "stats": stats })))
}

/// Extracts a topic id from a heartbeat path shaped like `/forum/topics/<uuid>`
/// (with optional trailing segments), or `None` for any other path.
fn topic_id_from_path(path: &str) -> Option<Uuid> {
    let rest = path.strip_prefix("/forum/topics/")?;
    let segment = rest.split('/').next().unwrap_or("");
    Uuid::parse_str(segment).ok()
}

/// GET /online/detailed — "who's online", with a best-effort readable location
/// (the topic the user is currently on) for each entry.
///
/// SECURITY: the location is only included when the *caller* can view the
/// forum the topic lives in (`PermissionService::effective`) — a member on a
/// restricted forum's topic must never leak that forum's existence or the
/// topic's title to callers who cannot see it (mirrors SEC-01/14). Any other
/// path (not a topic, or a topic that no longer resolves) simply carries no
/// location: the user still shows up as online, without detail.
pub async fn online_detailed(
    State(state): State<AppState>,
    Extension(user): Extension<ForumUser>,
) -> Result<Json<Value>> {
    let rows = PresenceService::who_online_with_path(5, &state.db).await?;
    let mut users = Vec::with_capacity(rows.len());
    for (user_id, path) in rows {
        let location = match path.as_deref().and_then(topic_id_from_path) {
            Some(topic_id) => match TopicService::get(topic_id, &state.db).await {
                Ok(topic) => {
                    let perms = PermissionService::effective(topic.forum_id, &user, &state.db).await?;
                    if perms.can_view { Some(topic.title) } else { None }
                }
                // Stale heartbeat pointing at a deleted/purged topic: no detail,
                // not a hard error for the whole page.
                Err(_) => None,
            },
            None => None,
        };
        users.push(json!({ "user_id": user_id, "path": location }));
    }
    Ok(Json(json!({ "users": users })))
}

/// GET /leaderboard — top contributors by post count.
pub async fn leaderboard(State(state): State<AppState>) -> Result<Json<Value>> {
    let top = StatsService::top_posters(20, &state.db).await?;
    let top: Vec<Value> = top
        .into_iter()
        .map(|(user_id, post_count)| json!({ "user_id": user_id, "post_count": post_count }))
        .collect();
    Ok(Json(json!({ "top": top })))
}

#[derive(Deserialize)]
pub struct MembersQuery {
    /// 'posts' (default) | 'recent' | 'active'.
    pub sort:   Option<String>,
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

/// GET /members — paginated members directory. "Members" here are participants:
/// anyone who has a forum profile (posted, reacted, or set a profile). The core
/// owns the full account list; the forum only knows who has taken part.
pub async fn members(State(state): State<AppState>, Query(q): Query<MembersQuery>) -> Result<Json<Value>> {
    let (limit, offset) = Pagination { limit: q.limit, offset: q.offset }.resolve(30, 100);
    let sort = q.sort.as_deref().unwrap_or("posts");
    let (members, total) = RankService::members_page(sort, limit, offset, &state.db).await?;
    Ok(Json(json!({ "members": members, "total": total })))
}

/// One row of the "team" page: a moderator, and the forum they moderate.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TeamMember {
    pub forum_id:   Uuid,
    pub forum_name: String,
    pub user_id:    Uuid,
}

/// GET /team — the moderation team, grouped by forum.
///
/// The core does not expose the platform's admin roster to a module, so this
/// deliberately lists per-forum moderators only ("team" page,
/// moderation flavor) — never platform admins. Restricted to forums the
/// caller may see, via the same `push_visible_forum` predicate every other
/// listing uses (SEC-01/14): a forum hidden from the caller never leaks its
/// moderators either.
pub async fn team(State(state): State<AppState>, Extension(user): Extension<ForumUser>) -> Result<Json<Value>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT m.forum_id, f.name AS forum_name, m.user_id \
         FROM forum.moderators m JOIN forum.forums f ON f.id = m.forum_id WHERE ",
    );
    PermissionService::push_visible_forum(&mut qb, "f.id", &user);
    qb.push(" ORDER BY f.position, f.name, m.created_at");
    let moderators = qb.build_query_as::<TeamMember>().fetch_all(&state.db).await?;
    Ok(Json(json!({ "moderators": moderators })))
}
