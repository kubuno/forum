//! Personal RSS/Atom feeds (see migration `000016`).
//!
//! SECURITY MODEL. Feed readers cannot log in, so the anonymous
//! `/public/feeds/:token/...` routes authenticate with an unguessable token in
//! the URL — the same capability-token pattern the calendar module uses for its
//! ICS feed. Two rules keep this from leaking anything:
//!
//!  1. A feed only ever contains topics its OWNER may see. Visibility is decided
//!     by the very same `PermissionService::push_visible_forum` used everywhere
//!     else, and the owner is always evaluated as an ORDINARY MEMBER — never an
//!     administrator — so a feed can never surface more than a normal member of
//!     that account would see in the app (a strictly conservative choice).
//!  2. There is deliberately NO token-less anonymous feed. This is a self-hosted
//!     workspace with no "guest" role; exposing member-visible content to the
//!     open internet is never a safe default, so it simply isn't offered.
//!
//! All user-supplied text is XML-escaped before it enters the document.

use base64::Engine;
use chrono::SecondsFormat;
use rand::RngCore;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    middleware::ForumUser,
    models::feed::{FeedEntry, FeedToken},
    services::{directory, permission_service::PermissionService},
    state::AppState,
};

pub struct FeedService;

impl FeedService {
    /// Mints a fresh, unguessable token (144 bits of entropy, URL-safe).
    fn new_token() -> String {
        let mut bytes = [0u8; 18];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    /// `POST /me/feed-tokens` — creates a feed URL for the caller.
    pub async fn create_token(user_id: Uuid, label: Option<String>, db: &PgPool) -> Result<FeedToken> {
        let token = Self::new_token();
        let label = label.and_then(|l| {
            let t = l.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        });
        let row: FeedToken = sqlx::query_as(
            "INSERT INTO forum.feed_tokens (token, user_id, label)
             VALUES ($1, $2, $3)
             RETURNING token, user_id, label, created_at, last_used_at",
        )
        .bind(&token)
        .bind(user_id)
        .bind(label)
        .fetch_one(db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "failed to create feed token");
            e
        })?;
        Ok(row)
    }

    /// `GET /me/feed-tokens` — the caller's own tokens, newest first.
    pub async fn list_tokens(user_id: Uuid, db: &PgPool) -> Result<Vec<FeedToken>> {
        let rows: Vec<FeedToken> = sqlx::query_as(
            "SELECT token, user_id, label, created_at, last_used_at
               FROM forum.feed_tokens
              WHERE user_id = $1
              ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// `DELETE /me/feed-tokens/:token` — revokes one. Keyed on `user_id`, so a
    /// member can only ever revoke a token they own.
    pub async fn revoke_token(user_id: Uuid, token: &str, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM forum.feed_tokens WHERE user_id = $1 AND token = $2")
            .bind(user_id)
            .bind(token)
            .execute(db)
            .await?;
        Ok(())
    }

    /// Resolves a feed token to its owner and stamps `last_used_at`. Returns
    /// `None` for an unknown token — the public handler then answers 404, never
    /// distinguishing "no such token" from any other miss.
    pub async fn resolve(token: &str, db: &PgPool) -> Result<Option<Uuid>> {
        let user_id: Option<Uuid> = sqlx::query_scalar(
            "UPDATE forum.feed_tokens SET last_used_at = NOW()
              WHERE token = $1
              RETURNING user_id",
        )
        .bind(token)
        .fetch_optional(db)
        .await?;
        Ok(user_id)
    }

    /// The most recent topics the owner may currently see (approved, not
    /// deleted), across every forum visible to them — evaluated as an ordinary
    /// member. Capped at 30.
    pub async fn recent_topics_for(state: &AppState, user_id: Uuid) -> Result<Vec<FeedEntry>> {
        // Conservative viewer: the token owner as a plain member, never admin —
        // but carrying their real groups so the feed honours the same additive
        // per-group grants the app does.
        let group_ids = directory::user_group_ids(state, user_id).await;
        let viewer = ForumUser {
            id: user_id,
            role: "user".to_string(),
            email: String::new(),
            group_ids,
        };
        let db = &state.db;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT t.id AS topic_id, t.forum_id AS forum_id, t.title AS title, \
                    t.author_id AS author_id, \
                    COALESCE(t.last_post_at, t.created_at) AS updated_at \
               FROM forum.topics t JOIN forum.forums f ON f.id = t.forum_id \
              WHERE t.is_deleted = FALSE AND t.is_approved = TRUE AND ",
        );
        PermissionService::push_visible_forum(&mut qb, "f.id", &viewer);
        qb.push(" ORDER BY updated_at DESC LIMIT 30");

        let rows: Vec<FeedEntry> = qb.build_query_as().fetch_all(db).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "failed to build feed");
            ForumError::from(e)
        })?;
        Ok(rows)
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    /// Escapes the five XML predefined entities. Applied to every piece of
    /// user-controlled text (topic titles, author names) before it is written
    /// into the document.
    fn xml_escape(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&apos;"),
                _ => out.push(c),
            }
        }
        out
    }

    /// `{base}/forum/topics/{id}` — an absolute link when the request carried a
    /// host, a root-relative one otherwise (still valid inside the shell).
    fn topic_link(base: &str, topic_id: Uuid) -> String {
        format!("{base}/forum/topics/{topic_id}")
    }

    /// Resolves author display names once per unique author for the whole feed.
    async fn author_names(state: &AppState, entries: &[FeedEntry]) -> std::collections::HashMap<Uuid, String> {
        let mut names = std::collections::HashMap::new();
        for e in entries {
            if names.contains_key(&e.author_id) {
                continue;
            }
            let name = directory::display_name(state, e.author_id)
                .await
                .unwrap_or_else(|| "Membre".to_string());
            names.insert(e.author_id, name);
        }
        names
    }

    /// Atom 1.0 document. `base` is the request origin (may be empty).
    pub async fn render_atom(state: &AppState, base: &str, entries: &[FeedEntry]) -> String {
        let names = Self::author_names(state, entries).await;
        let updated = entries
            .first()
            .map(|e| e.updated_at.to_rfc3339_opts(SecondsFormat::Secs, true))
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
        let self_link = format!("{base}/forum");

        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        out.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
        out.push_str("  <title>Forum</title>\n");
        out.push_str(&format!("  <link href=\"{}\"/>\n", Self::xml_escape(&self_link)));
        // A stable, location-independent feed id (the self link may be
        // root-relative, which is not a valid atom:id).
        out.push_str("  <id>urn:kubuno-forum:feed</id>\n");
        out.push_str(&format!("  <updated>{updated}</updated>\n"));
        for e in entries {
            let link = Self::topic_link(base, e.topic_id);
            let author = names.get(&e.author_id).map(String::as_str).unwrap_or("Membre");
            out.push_str("  <entry>\n");
            out.push_str(&format!("    <title>{}</title>\n", Self::xml_escape(&e.title)));
            out.push_str(&format!("    <link href=\"{}\"/>\n", Self::xml_escape(&link)));
            out.push_str(&format!(
                "    <id>urn:kubuno-forum:topic:{}</id>\n",
                e.topic_id
            ));
            out.push_str(&format!(
                "    <updated>{}</updated>\n",
                e.updated_at.to_rfc3339_opts(SecondsFormat::Secs, true)
            ));
            out.push_str(&format!("    <author><name>{}</name></author>\n", Self::xml_escape(author)));
            out.push_str("  </entry>\n");
        }
        out.push_str("</feed>\n");
        out
    }

    /// RSS 2.0 document. `base` is the request origin (may be empty).
    pub async fn render_rss(state: &AppState, base: &str, entries: &[FeedEntry]) -> String {
        let names = Self::author_names(state, entries).await;
        let self_link = format!("{base}/forum");

        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        out.push_str("<rss version=\"2.0\">\n<channel>\n");
        out.push_str("  <title>Forum</title>\n");
        out.push_str(&format!("  <link>{}</link>\n", Self::xml_escape(&self_link)));
        out.push_str("  <description>Sujets récents du forum</description>\n");
        for e in entries {
            let link = Self::topic_link(base, e.topic_id);
            let author = names.get(&e.author_id).map(String::as_str).unwrap_or("Membre");
            out.push_str("  <item>\n");
            out.push_str(&format!("    <title>{}</title>\n", Self::xml_escape(&e.title)));
            out.push_str(&format!("    <link>{}</link>\n", Self::xml_escape(&link)));
            out.push_str(&format!(
                "    <guid isPermaLink=\"false\">urn:kubuno-forum:topic:{}</guid>\n",
                e.topic_id
            ));
            out.push_str(&format!(
                "    <pubDate>{}</pubDate>\n",
                e.updated_at.to_rfc2822()
            ));
            out.push_str(&format!("    <dc:creator xmlns:dc=\"http://purl.org/dc/elements/1.1/\">{}</dc:creator>\n", Self::xml_escape(author)));
            out.push_str("  </item>\n");
        }
        out.push_str("</channel>\n</rss>\n");
        out
    }
}
