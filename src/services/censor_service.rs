//! Server-side word censor: an admin-curated list of words is
//! substituted in post bodies at render time. The substitution happens on the
//! server before a post is serialized, so a client has no toggle to bypass
//! it — unlike a client-side filter, it cannot see the raw word at all.
//!
//! Patterns are compiled once into regexes and cached in memory (`CENSOR`),
//! recompiled whenever the list changes (`reload`, called after every write)
//! and once at boot (see `main.rs`). `apply` only reads the cache — no
//! database round trip — so it is cheap enough to call on every rendered
//! post.
//!
//! SEC: `pattern` is always treated as a literal word/phrase, never as a
//! regex fragment — it is escaped with `regex::escape` before being compiled
//! into `(?i)\b<escaped>\b`. Never build a regex directly from unescaped
//! admin input. The `regex` crate is linear-time (no backtracking), so even
//! an unescaped pattern could not cause ReDoS — but escaping still matters:
//! without it, an admin-entered `.` or `(` would behave as regex syntax
//! instead of a literal character, silently censoring the wrong things.

use std::sync::{LazyLock, RwLock};

use regex::Regex;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{ForumError, Result},
    models::moderation::{CensoredWord, CreateCensoredWordDto},
};

/// Compiled (pattern, replacement) pairs, rebuilt from `forum.censored_words`
/// by `reload`. Empty until the first successful reload — `apply` is then a
/// no-op, never a panic.
static CENSOR: LazyLock<RwLock<Vec<(Regex, String)>>> = LazyLock::new(|| RwLock::new(Vec::new()));

pub struct CensorService;

impl CensorService {
    /// Replaces every occurrence of a censored word in `text` (case
    /// insensitive, word boundaries) using the in-memory cache. Read-only —
    /// never touches the database — safe to call on every post rendered in a
    /// listing. Falls back to the original text if the cache lock is
    /// poisoned (never expected, but this must not panic a request).
    pub fn apply(text: &str) -> String {
        let guard = match CENSOR.read() {
            Ok(g) => g,
            Err(e) => {
                tracing::error!(error = %e, "censor cache: lock poisoned on read");
                return text.to_string();
            }
        };
        if guard.is_empty() {
            return text.to_string();
        }
        let mut out = text.to_string();
        for (re, replacement) in guard.iter() {
            out = re.replace_all(&out, replacement.as_str()).into_owned();
        }
        out
    }

    /// Recompiles the in-memory cache from the database. A pattern that fails
    /// to compile (defensive check only — patterns are escaped literals, so
    /// this should not happen in practice) is skipped and logged rather than
    /// failing the whole reload.
    pub async fn reload(db: &PgPool) -> Result<()> {
        let words = Self::list(db).await?;
        let mut compiled = Vec::with_capacity(words.len());
        for w in &words {
            let pattern = w.pattern.trim();
            if pattern.is_empty() {
                continue;
            }
            let expr = format!(r"(?i)\b{}\b", regex::escape(pattern));
            match Regex::new(&expr) {
                Ok(re) => compiled.push((re, w.replacement.clone())),
                Err(e) => tracing::warn!(id = %w.id, pattern = %w.pattern, error = %e, "censored word: invalid pattern, skipped"),
            }
        }
        match CENSOR.write() {
            Ok(mut guard) => *guard = compiled,
            Err(e) => tracing::error!(error = %e, "censor cache: lock poisoned on write"),
        }
        Ok(())
    }

    pub async fn list(db: &PgPool) -> Result<Vec<CensoredWord>> {
        let rows = sqlx::query_as::<_, CensoredWord>(
            "SELECT * FROM forum.censored_words ORDER BY created_at",
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(dto: CreateCensoredWordDto, db: &PgPool) -> Result<CensoredWord> {
        let replacement = dto
            .replacement
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("***");
        let word = sqlx::query_as::<_, CensoredWord>(
            "INSERT INTO forum.censored_words (pattern, replacement) VALUES ($1, $2) RETURNING *",
        )
        .bind(dto.pattern.trim())
        .bind(replacement)
        .fetch_one(db)
        .await?;
        Self::reload(db).await?;
        Ok(word)
    }

    pub async fn delete(id: Uuid, db: &PgPool) -> Result<()> {
        let r = sqlx::query("DELETE FROM forum.censored_words WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        if r.rows_affected() == 0 {
            return Err(ForumError::NotFound(format!("Censored word {id}")));
        }
        Self::reload(db).await?;
        Ok(())
    }
}
