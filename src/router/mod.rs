use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

use crate::{
    handlers::{
        attachments, bookmarks, categories, community, discovery, drafts, faq, feed, forums,
        health, ignore, moderation, notifications, permissions, pm, polls, posts, profile_fields,
        ranks, reactions, search, tags, topics,
    },
    middleware::{enforce_ban, rate_limit_writes, require_auth},
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Categories
        .route("/categories",     get(categories::list).post(categories::create))
        .route("/categories/:id", get(categories::get).patch(categories::update).delete(categories::delete))
        // Forums
        .route("/forums",         get(forums::list).post(forums::create))
        .route("/forums/reorder", patch(forums::reorder))
        .route("/forums/:id",     get(forums::get).patch(forums::update).delete(forums::delete))
        .route("/forums/:id/topics",     get(topics::list_by_forum).post(topics::create))
        .route("/forums/:id/read-state", get(forums::read_state))
        .route("/forums/:id/read-all",   post(forums::read_all))
        .route("/forums/:id/subscribe",  post(forums::subscribe).delete(forums::unsubscribe))
        .route("/forums/:id/moderators", get(moderation::list_moderators).post(moderation::add_moderator))
        .route("/forums/:id/moderators/:uid", delete(moderation::remove_moderator))
        .route("/forums/:id/permissions", get(permissions::list).put(permissions::set))
        .route("/forums/:id/group-permissions", get(permissions::list_group).put(permissions::set_group))
        .route("/groups", get(permissions::list_groups))
        .route("/forums/:id/prune",       post(moderation::prune_forum))
        // Topics
        .route("/topics/:id",       get(topics::get).patch(topics::update).delete(topics::delete))
        .route("/topics/:id/lock",   post(topics::lock))
        .route("/topics/:id/unlock", post(topics::unlock))
        .route("/topics/:id/restore", post(moderation::restore_topic))
        .route("/topics/:id/purge",   delete(moderation::purge_topic))
        .route("/topics/:id/move",   post(topics::move_topic))
        .route("/topics/:id/split",  post(topics::split))
        .route("/topics/:id/merge",  post(topics::merge))
        .route("/topics/:id/posts",  get(posts::list).post(posts::create))
        .route("/topics/:id/read",   post(topics::mark_read))
        .route("/topics/:id/read-state", get(topics::read_state))
        .route("/topics/:id/subscribe", post(topics::subscribe).delete(topics::unsubscribe))
        .route("/topics/:id/bookmark",  post(bookmarks::toggle))
        .route("/topics/:id/reactions", get(reactions::for_topic))
        .route("/topics/:id/solution",  post(topics::set_solution).delete(topics::clear_solution))
        .route("/topics/:id/tags",      get(tags::for_topic).put(tags::set_for_topic))
        .route("/topics/:id/poll",      get(polls::get))
        // Posts
        .route("/posts/:id",             get(posts::get).patch(posts::update).delete(posts::delete))
        .route("/posts/:id/revisions",   get(posts::list_revisions))
        .route("/posts/:id/react",       post(reactions::react))
        .route("/posts/:id/reactions/users", get(reactions::users_for_post))
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
        .route("/online",          get(community::online))
        .route("/online/detailed", get(community::online_detailed))
        .route("/stats",           get(community::stats))
        .route("/leaderboard",     get(community::leaderboard))
        .route("/members",         get(community::members))
        .route("/team",            get(community::team))
        .route("/me/heartbeat",   post(community::heartbeat))
        .route("/me/read-all",    post(community::read_all))
        // Moderation queue + tooling
        .route("/reports",     get(moderation::list_reports))
        .route("/reports/:id", patch(moderation::resolve_report))
        .route("/report-reasons",     get(moderation::list_report_reasons).post(moderation::create_report_reason))
        .route("/report-reasons/:id", delete(moderation::delete_report_reason))
        .route("/censored-words",     get(moderation::list_censored_words).post(moderation::create_censored_word))
        .route("/censored-words/:id", delete(moderation::delete_censored_word))
        .route("/trash",                 get(moderation::list_trash))
        .route("/mod/queue",             get(moderation::pending_queue))
        .route("/mod/queue/:id/approve", post(moderation::approve_pending))
        .route("/mod/queue/:id/reject",  post(moderation::reject_pending))
        .route("/mod/topics/bulk", post(moderation::bulk_topics))
        .route("/mod/log",     get(moderation::mod_log))
        .route("/mod/bans",    get(moderation::list_bans))
        // IP / email bans (phpBB-style, exact match, admin only) — enforced by
        // `middleware::enforce_ban`, layered below.
        .route("/bans/ip",        get(moderation::list_ip_bans).post(moderation::ban_ip))
        .route("/bans/ip/:id",    delete(moderation::unban_ip))
        .route("/bans/email",     get(moderation::list_email_bans).post(moderation::ban_email))
        .route("/bans/email/:id", delete(moderation::unban_email))
        .route("/mod/notes",   post(moderation::add_note))
        .route("/mod/users/:uid/warn",     post(moderation::warn_user))
        .route("/mod/users/:uid/warnings", get(moderation::list_warnings))
        .route("/mod/users/:uid/ban",      post(moderation::ban_user).delete(moderation::unban_user))
        .route("/mod/users/:uid/notes",    get(moderation::list_notes))
        // Ranks & profiles
        .route("/ranks",     get(ranks::list).post(ranks::create))
        .route("/ranks/:id", patch(ranks::update).delete(ranks::delete))
        .route("/profiles/brief", get(ranks::brief_profiles))
        .route("/profiles/:uid", get(ranks::get_profile))
        .route("/profiles/:uid/activity", get(ranks::activity))
        .route("/profiles/:uid/rank", patch(ranks::assign_rank))
        .route("/me/profile",       get(ranks::my_profile).patch(ranks::update_my_signature))
        .route("/me/subscriptions", get(ranks::my_subscriptions))
        // Custom profile fields (phpBB-style EAV): admin-curated definitions,
        // one member's own answers, and a member's answers as shown to
        // others. `/me/profile-fields` is a distinct literal segment from
        // `/profile-fields/:id` and `/users/:id/profile-fields`, so none of
        // them can shadow another.
        .route("/profile-fields",     get(profile_fields::list_fields).post(profile_fields::create_field))
        .route("/profile-fields/:id", patch(profile_fields::update_field).delete(profile_fields::delete_field))
        .route("/me/profile-fields",  get(profile_fields::my_values).put(profile_fields::set_my_values))
        .route("/users/:id/profile-fields", get(profile_fields::user_values))
        // FAQ (phpBB-style): admin-curated question/answer pairs, read by
        // every member, written only by an admin (see `handlers/faq.rs`).
        .route("/faq",     get(faq::list).post(faq::create))
        .route("/faq/:id", patch(faq::update).delete(faq::delete))
        .route("/me/bookmarks",     get(bookmarks::list))
        .route("/me/drafts",        get(drafts::list).put(drafts::save))
        .route("/me/drafts/:id",    delete(drafts::delete))
        .route("/me/notifications", get(notifications::list))
        .route("/me/notifications/read", post(notifications::mark_read))
        // Ignore list (phpBB "foes"/zebra) — distinct from `/me/pm/blocks`,
        // which mutes PMs; this one only affects the topic view's display.
        .route("/me/ignored",     get(ignore::list).post(ignore::add))
        .route("/me/ignored/:uid", delete(ignore::remove))
        // Private messages (modern conversations, not the phpBB PM system).
        // ⚠️ `/me/pm/unread` and `/me/pm/blocks` are registered before
        // `/me/pm/:id` so they are never captured by the `:id` param.
        .route("/me/pm",         get(pm::list_threads).post(pm::create_thread))
        .route("/me/pm/unread",  get(pm::unread_count))
        .route("/me/pm/blocks",  get(pm::list_blocks).post(pm::create_block))
        .route("/me/pm/blocks/:uid", delete(pm::delete_block))
        .route("/me/pm/:id",      get(pm::get_thread).post(pm::send_message).delete(pm::delete_thread))
        .route("/me/pm/:id/read", post(pm::mark_read))
        // Personal RSS/Atom feed tokens (the feed documents themselves are
        // served by the anonymous `public` router below).
        .route("/me/feed-tokens", get(feed::list_tokens).post(feed::create_token))
        .route("/me/feed-tokens/:token", delete(feed::revoke_token))
        // Search
        .route("/search", get(search::search))
        // Layer order matters: axum runs the LAST `.layer()` call FIRST (it is the
        // outermost). `require_auth` must stay outermost (it populates `ForumUser`),
        // `enforce_ban` runs right after it (it reads `ForumUser` + the client-IP
        // header to reject a banned account/IP/email — SEC), and the write rate
        // limiter sits innermost, closest to the handlers (SEC-09). Execution order
        // is therefore: require_auth → enforce_ban → rate_limit_writes → handler.
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_writes))
        .layer(middleware::from_fn(enforce_ban))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state.clone());

    let system = Router::new()
        .route("/health", get(health::health))
        .with_state(state.clone());

    // Anonymous, token-authenticated feeds. Deliberately OUTSIDE `require_auth`
    // (a feed reader cannot carry a session): the capability token in the URL is
    // the sole credential, and a feed only ever contains topics its owner may
    // see, evaluated as an ordinary member. See `services/feed_service.rs`.
    let public = Router::new()
        .route("/public/feeds/:token/atom.xml", get(feed::public_atom))
        .route("/public/feeds/:token/rss.xml", get(feed::public_rss))
        .with_state(state);

    Router::new()
        .merge(system)
        .merge(public)
        .merge(authed)
        // Cap request bodies before they are buffered: the per-field validators
        // bound text length, but nothing else stops a multi-megabyte payload from
        // being read into memory first (SEC-08). 2 MiB comfortably covers the
        // longest allowed post plus its JSON envelope.
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        // No CORS layer: this module only ever receives requests from the core's
        // server-side proxy on loopback, never a cross-origin browser call, so a
        // permissive CORS policy was only ever attack surface if the port leaked
        // (SEC-23). Restore a strict `CorsLayer` here if the module is ever meant
        // to be reachable directly by a browser.
        .layer(TraceLayer::new_for_http())
}
