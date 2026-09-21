//! Runs the forum module's own migrations and the primitives the kubuno-db port
//! changed against a real server of **each** engine, from a single compiled
//! binary — the proof that the engine is a run-time choice, not a build-time
//! one.
//!
//! The suite issues the very SQL the services issue (ids generated in Rust,
//! `title_norm`/`body_norm` from `search::normalize`, dialect upserts) and calls
//! the pure service helpers directly (`aggregates::recompute_*`,
//! `SearchService::search`). It deliberately avoids the HTTP/core-dependent
//! paths.
//!
//! * SQLite always runs (a temp file, no server).
//! * PostgreSQL runs when `KUBUNO_PG_TEST_URL` points at a throwaway database.
//! * MySQL/MariaDB runs when `KUBUNO_MYSQL_TEST_URL` does.
//!
//! ```sh
//! KUBUNO_PG_TEST_URL=postgres://u:p@127.0.0.1:5432/kubuno_test \
//! KUBUNO_MYSQL_TEST_URL=mysql://u:p@127.0.0.1:3306/forum \
//!   SQLX_OFFLINE=true cargo test --test db_portability
//! ```

use kubuno_db::dialect::{Assign, SqlType};
use kubuno_db::search::normalize;
use kubuno_db::{new_id, params, DbPool};
use kubuno_forum::middleware::ForumUser;
use kubuno_forum::services::aggregates;
use kubuno_forum::services::search_service::{SearchFilters, SearchService};
use kubuno_forum::SCHEMA;
use uuid::Uuid;

fn base_settings(engine: &str) -> kubuno_db::DbSettings {
    kubuno_db::DbSettings {
        engine: engine.to_string(),
        url: None,
        host: None,
        port: None,
        user: None,
        password: None,
        database: None,
        path: None,
        max_connections: 4,
        min_connections: 0,
        connect_timeout: std::time::Duration::from_secs(10),
        run_migrations: true,
    }
}

/// Migrations run one at a time: the PostgreSQL and MySQL suites may share a server.
static EXCLUSIVE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn migrated_pool(settings: kubuno_db::DbSettings) -> (DbPool, impl Sized) {
    let guard = EXCLUSIVE.lock().await;
    let pool = kubuno_db::connect(&settings, SCHEMA).await.expect("connect");
    kubuno_db::migrations!(
        "./migrations/postgres",
        "./migrations/mysql",
        "./migrations/sqlite",
    )
    .run(&pool, SCHEMA)
    .await
    .expect("migrations");
    (pool, guard)
}

fn admin(id: Uuid) -> ForumUser {
    ForumUser { id, role: "admin".to_string(), email: String::new(), group_ids: Vec::new() }
}

async fn count(pool: &DbPool, sql: &str, binds: Vec<kubuno_db::DbValue>) -> i64 {
    pool.fetch_scalar::<i64>(sql, binds).await.expect("count")
}

/// Reads an `INT` column (int4 on PostgreSQL, which refuses an i64 decode) at
/// its own width, so the same read works on all three engines.
async fn count_int(pool: &DbPool, sql: &str, binds: Vec<kubuno_db::DbValue>) -> i32 {
    pool.fetch_scalar::<i32>(sql, binds).await.expect("count_int")
}

async fn insert_category(pool: &DbPool) -> Uuid {
    let id = new_id();
    pool.execute(
        "INSERT INTO forum.categories (id, name, description, position) VALUES ($1, $2, $3, $4)",
        params![id, "Test", "desc", 0i32],
    )
    .await
    .expect("insert category");
    id
}

async fn insert_forum(pool: &DbPool, category_id: Uuid) -> Uuid {
    let id = new_id();
    pool.execute(
        "INSERT INTO forum.forums (id, category_id, name, description, position) VALUES ($1, $2, $3, $4, $5)",
        params![id, category_id, "General", "desc", 0i32],
    )
    .await
    .expect("insert forum");
    id
}

async fn insert_topic(pool: &DbPool, forum_id: Uuid, author: Uuid, title: &str) -> Uuid {
    let id = new_id();
    pool.execute(
        "INSERT INTO forum.topics (id, forum_id, author_id, title, slug, topic_type, title_norm) \
         VALUES ($1, $2, $3, $4, $5, 'normal', $6)",
        params![id, forum_id, author, title, aggregates::slugify(title), normalize(title)],
    )
    .await
    .expect("insert topic");
    id
}

async fn insert_post(pool: &DbPool, topic_id: Uuid, forum_id: Uuid, author: Uuid, body: &str, first: bool) -> Uuid {
    let id = new_id();
    pool.execute(
        "INSERT INTO forum.posts (id, topic_id, forum_id, author_id, body_md, is_first_post, body_norm) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        params![id, topic_id, forum_id, author, body, first, normalize(body)],
    )
    .await
    .expect("insert post");
    id
}

async fn full_suite(pool: &DbPool) {
    let author = new_id();
    let category_id = insert_category(pool).await;
    let forum_id = insert_forum(pool, category_id).await;
    let topic_id = insert_topic(pool, forum_id, author, "Bienvenue sur le forum").await;
    let _first = insert_post(pool, topic_id, forum_id, author, "Bonjour le café du village", true).await;
    let _reply = insert_post(pool, topic_id, forum_id, author, "Une réponse à propos des chevaux", false).await;

    // ── Aggregates (recompute reads the source rows, writes from Rust) ────────
    let mut tx = pool.begin().await.expect("begin");
    aggregates::recompute_topic(&mut tx, topic_id).await.expect("recompute topic");
    aggregates::recompute_forum(&mut tx, forum_id).await.expect("recompute forum");
    tx.commit().await.expect("commit");

    let reply_count = count_int(
        pool,
        "SELECT reply_count FROM forum.topics WHERE id = $1",
        params![topic_id],
    )
    .await;
    assert_eq!(reply_count, 1, "two approved posts → one reply");
    let post_count = count_int(
        pool,
        "SELECT post_count FROM forum.forums WHERE id = $1",
        params![forum_id],
    )
    .await;
    assert_eq!(post_count, 2, "forum post_count denormalised from source rows");

    // ── Upsert idempotency: reaction (UNIQUE post_id,user_id,emoji) ───────────
    let b = pool.backend();
    let post_id = _first;
    let react_sql = format!(
        "INSERT {}INTO forum.reactions (id, post_id, user_id, emoji) VALUES ($1, $2, $3, $4){}",
        b.insert_ignore_prefix(),
        b.on_conflict_do_nothing(&["post_id", "user_id", "emoji"]),
    );
    for _ in 0..2 {
        pool.execute(&react_sql, params![new_id(), post_id, author, "👍"]).await.expect("react");
    }
    let reactions = count(pool, "SELECT COUNT(*) FROM forum.reactions WHERE post_id = $1", params![post_id]).await;
    assert_eq!(reactions, 1, "second identical reaction is a no-op");

    // ── Plain UNIQUE on a nullable pair: a topic-sub and a forum-sub coexist ──
    let sub_sql = |cols: &[&'static str]| {
        format!(
            "INSERT {}INTO forum.subscriptions (id, user_id, topic_id, forum_id) VALUES ($1, $2, $3, $4){}",
            b.insert_ignore_prefix(),
            b.on_conflict_do_nothing(cols),
        )
    };
    pool.execute(
        &sub_sql(&["user_id", "topic_id"]),
        params![new_id(), author, topic_id, None::<Uuid>],
    )
    .await
    .expect("sub topic");
    pool.execute(
        &sub_sql(&["user_id", "forum_id"]),
        params![new_id(), author, None::<Uuid>, forum_id],
    )
    .await
    .expect("sub forum");
    let subs = count(pool, "SELECT COUNT(*) FROM forum.subscriptions WHERE user_id = $1", params![author]).await;
    assert_eq!(subs, 2, "NULLs are distinct: topic-sub and forum-sub don't collide");

    // ── Upsert DO UPDATE: a per-forum permission row ─────────────────────────
    let perm_sql = format!(
        "INSERT INTO forum.permissions (id, forum_id, role, can_view, can_post, can_reply, can_attach) \
         VALUES ($1, $2, $3, $4, $5, $6, $7){}",
        b.upsert(
            "permissions",
            &["forum_id", "role"],
            &[Assign::Incoming("can_view"), Assign::Incoming("can_post")],
        )
    );
    pool.execute(&perm_sql, params![new_id(), forum_id, "user", true, true, true, true]).await.expect("perm 1");
    pool.execute(&perm_sql, params![new_id(), forum_id, "user", false, false, true, true]).await.expect("perm 2");
    let perms = count(pool, "SELECT COUNT(*) FROM forum.permissions WHERE forum_id = $1", params![forum_id]).await;
    assert_eq!(perms, 1, "upsert on (forum_id, role) keeps one row");
    let can_view = count(
        pool,
        &format!(
            "SELECT {} FROM forum.permissions WHERE forum_id = $1 AND role = 'user' AND can_view = FALSE",
            b.cast("1", SqlType::BigInt)
        ),
        params![forum_id],
    )
    .await;
    assert_eq!(can_view, 1, "the second upsert updated can_view to FALSE");

    // ── Portable search: normalized (stemmed + deaccented) LIKE ──────────────
    let user = admin(author);
    let hits = SearchService::search(&user, "cafe", &SearchFilters::default(), 20, 0, pool)
        .await
        .expect("search cafe");
    assert!(
        hits.iter().any(|h| h.post_id == _first),
        "accent-folded query 'cafe' finds the post 'café'"
    );
    let title_hits = SearchService::search(&user, "bienvenue", &SearchFilters::default(), 20, 0, pool)
        .await
        .expect("search title");
    assert!(!title_hits.is_empty(), "title term matches via title_norm");
    let miss = SearchService::search(&user, "zzznothing", &SearchFilters::default(), 20, 0, pool)
        .await
        .expect("search miss");
    assert!(miss.is_empty(), "a term present nowhere matches nothing");
}

#[tokio::test]
async fn sqlite_from_the_one_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = base_settings("sqlite");
    s.path = Some(dir.path().to_string_lossy().into_owned());
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn postgres_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_PG_TEST_URL") else {
        eprintln!("skipping: KUBUNO_PG_TEST_URL not set");
        return;
    };
    let mut s = base_settings("postgres");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn mysql_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_MYSQL_TEST_URL") else {
        eprintln!("skipping: KUBUNO_MYSQL_TEST_URL not set");
        return;
    };
    let mut s = base_settings("mysql");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}
