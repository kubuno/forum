use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{
        attachments, bookmarks, categories, community, discovery, drafts, forums, health,
        moderation, notifications, permissions, polls, posts, ranks, reactions, search, tags,
        topics,
    },
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Categories
        .route("/categories",     get(categories::list).post(categories::create))
        .route("/categories/:id", get(categories::get).patch(categories::update).delete(categories::delete))
        // Forums
        .route("/forums",     get(forums::list).post(forums::create))
        .route("/forums/:id", get(forums::get).patch(forums::update).delete(forums::delete))
        .route("/forums/:id/topics",     get(topics::list_by_forum).post(topics::create))
        .route("/forums/:id/read-state", get(forums::read_state))
        .route("/forums/:id/subscribe",  post(forums::subscribe).delete(forums::unsubscribe))
        .route("/forums/:id/moderators", get(moderation::list_moderators).post(moderation::add_moderator))
        .route("/forums/:id/moderators/:uid", delete(moderation::remove_moderator))
        .route("/forums/:id/permissions", get(permissions::list).put(permissions::set))
        // Topics
        .route("/topics/:id",       get(topics::get).patch(topics::update).delete(topics::delete))
        .route("/topics/:id/lock",   post(topics::lock))
        .route("/topics/:id/unlock", post(topics::unlock))
        .route("/topics/:id/move",   post(topics::move_topic))
        .route("/topics/:id/split",  post(topics::split))
        .route("/topics/:id/merge",  post(topics::merge))
        .route("/topics/:id/posts",  get(posts::list).post(posts::create))
        .route("/topics/:id/read",   post(topics::mark_read))
        .route("/topics/:id/subscribe", post(topics::subscribe).delete(topics::unsubscribe))
        .route("/topics/:id/bookmark",  post(bookmarks::toggle))
        .route("/topics/:id/reactions", get(reactions::for_topic))
        .route("/topics/:id/solution",  post(topics::set_solution).delete(topics::clear_solution))
        .route("/topics/:id/tags",      get(tags::for_topic).put(tags::set_for_topic))
        .route("/topics/:id/poll",      get(polls::get))
        // Posts
        .route("/posts/:id",             get(posts::get).patch(posts::update).delete(posts::delete))
        .route("/posts/:id/react",       post(reactions::react))
        .route("/posts/:id/report",      post(moderation::report_post))
        .route("/posts/:id/remove",      post(moderation::remove_post))
        .route("/posts/:id/restore",     post(moderation::restore_post))
        .route("/posts/:id/attachments", get(attachments::list).post(attachments::create))
        .route("/attachments/:id",       delete(attachments::delete))
        // Polls, tags, discovery
        .route("/polls/:id/vote", post(polls::vote))
        .route("/tags",           get(tags::list).post(tags::create))
        .route("/tags/:id",       delete(tags::delete))
        .route("/feed",           get(discovery::feed))
        // Community: presence, stats, members
        .route("/online",         get(community::online))
        .route("/stats",          get(community::stats))
        .route("/members",        get(community::members))
        .route("/me/heartbeat",   post(community::heartbeat))
        // Moderation queue + tooling
        .route("/reports",     get(moderation::list_reports))
        .route("/reports/:id", patch(moderation::resolve_report))
        .route("/mod/log",     get(moderation::mod_log))
        .route("/mod/bans",    get(moderation::list_bans))
        .route("/mod/notes",   post(moderation::add_note))
        .route("/mod/users/:uid/warn",     post(moderation::warn_user))
        .route("/mod/users/:uid/warnings", get(moderation::list_warnings))
        .route("/mod/users/:uid/ban",      post(moderation::ban_user).delete(moderation::unban_user))
        .route("/mod/users/:uid/notes",    get(moderation::list_notes))
        // Ranks & profiles
        .route("/ranks",     get(ranks::list).post(ranks::create))
        .route("/ranks/:id", patch(ranks::update).delete(ranks::delete))
        .route("/profiles/:uid", get(ranks::get_profile))
        .route("/profiles/:uid/activity", get(ranks::activity))
        .route("/me/profile",       get(ranks::my_profile).patch(ranks::update_my_signature))
        .route("/me/subscriptions", get(ranks::my_subscriptions))
        .route("/me/bookmarks",     get(bookmarks::list))
        .route("/me/drafts",        get(drafts::list).put(drafts::save))
        .route("/me/drafts/:id",    delete(drafts::delete))
        .route("/me/notifications", get(notifications::list))
        .route("/me/notifications/read", post(notifications::mark_read))
        // Search
        .route("/search", get(search::search))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state.clone());

    let system = Router::new()
        .route("/health", get(health::health))
        .with_state(state);

    Router::new()
        .merge(system)
        .merge(authed)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
