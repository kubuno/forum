use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{
        attachments, categories, forums, health, moderation, permissions, posts, ranks, search,
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
        // Posts
        .route("/posts/:id",             get(posts::get).patch(posts::update).delete(posts::delete))
        .route("/posts/:id/report",      post(moderation::report_post))
        .route("/posts/:id/attachments", get(attachments::list).post(attachments::create))
        .route("/attachments/:id",       delete(attachments::delete))
        // Moderation queue
        .route("/reports",     get(moderation::list_reports))
        .route("/reports/:id", patch(moderation::resolve_report))
        // Ranks & profiles
        .route("/ranks",     get(ranks::list).post(ranks::create))
        .route("/ranks/:id", patch(ranks::update).delete(ranks::delete))
        .route("/profiles/:uid", get(ranks::get_profile))
        .route("/me/profile",       get(ranks::my_profile).patch(ranks::update_my_signature))
        .route("/me/subscriptions", get(ranks::my_subscriptions))
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
