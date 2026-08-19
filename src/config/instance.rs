//! Instance-wide settings of the forum module, as the administrator left them in
//! the console.
//!
//! Declared by `module.toml`'s `[[settings]]`, stored in `core.settings`, and read
//! back here through `/internal/modules/forum/settings` — a module owns its own
//! schema and cannot read the core's tables, and a background worker has no user
//! token for the public config route. The module is named in the URL so the read
//! works whether the instance shares one master secret or a derived one per
//! module.
//!
//! Every field here is read by code that acts on it: a knob that changes nothing
//! is worse than an absent one.

use serde_json::Value;

/// How new contributions are gated before they become visible.
///
/// A `Copy` enum rather than a raw `String`: the whole config lives behind a
/// lock and is copied out on every read, and a value the console does not know
/// must fall back to the compiled default instead of being interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalMode {
    /// Everything is published immediately.
    None,
    /// Contributions from members below the "new member" post threshold wait for
    /// a moderator; established members publish immediately.
    NewMembers,
    /// Every contribution waits for a moderator.
    All,
}

impl ApprovalMode {
    fn parse(raw: Option<&str>, fallback: Self) -> Self {
        match raw {
            Some("none")        => Self::None,
            Some("new_members") => Self::NewMembers,
            Some("all")         => Self::All,
            _                   => fallback,
        }
    }

    /// Whether a contribution is published straight away.
    ///
    /// `post_count` is the author's tally in `forum.user_profiles`. Moderators
    /// and administrators never queue — a queue nobody can be trusted past is a
    /// forum nobody can run.
    pub fn publishes_immediately(self, is_moderator: bool, post_count: i32, new_member_below: i32) -> bool {
        if is_moderator {
            return true;
        }
        match self {
            Self::None       => true,
            Self::All        => false,
            Self::NewMembers => post_count >= new_member_below,
        }
    }

    /// Whether the queue is armed at all — lets callers skip the profile lookup
    /// on the overwhelmingly common "no moderation" instance.
    pub fn is_active(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None       => "none",
            Self::NewMembers => "new_members",
            Self::All        => "all",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InstanceConfig {
    // ── Publication ─────────────────────────────────────────────────────────
    /// Ceiling, in characters, on a message body. A longer message is refused.
    pub max_post_length: usize,
    /// How long an author may keep editing their own message, in minutes.
    /// `0` = forever. Moderators are never bound by it.
    pub post_edit_window_minutes: i64,
    /// Minimum delay between two contributions by the same author, in seconds.
    /// `0` = no flood control.
    pub min_seconds_between_posts: i64,

    // ── Moderation ──────────────────────────────────────────────────────────
    /// Which contributions wait for a moderator before becoming visible.
    pub post_approval_mode: ApprovalMode,
    /// Post count below which an author still counts as a new member, for
    /// `ApprovalMode::NewMembers`.
    pub new_member_post_count: i32,
    /// Days a contribution may sit unapproved before the cleaner discards it.
    /// `0` = keep pending contributions forever.
    pub pending_retention_days: i32,

    // ── Profiles ────────────────────────────────────────────────────────────
    /// Whether members may attach a signature to their messages.
    pub allow_signatures: bool,
    /// Ceiling, in characters, on a signature.
    pub max_signature_length: usize,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            max_post_length:          100_000,
            post_edit_window_minutes: 0,
            min_seconds_between_posts: 0,
            post_approval_mode:       ApprovalMode::None,
            new_member_post_count:    5,
            pending_retention_days:   14,
            allow_signatures:         true,
            max_signature_length:     2_000,
        }
    }
}

impl InstanceConfig {
    /// Maps the core's `{key: value}` object onto the struct. Every read falls
    /// back to the compiled default rather than to a permissive value; an
    /// out-of-range number is treated as a mistake and ignored the same way.
    /// `0` is MEANINGFUL for the three windows (no limit / no flood control /
    /// never discard) and is therefore accepted rather than floored away.
    pub fn from_settings(settings: &Value) -> Self {
        let d = Self::default();
        let int_in = |key: &str, min: i64, max: i64, fallback: i64| -> i64 {
            settings
                .get(key)
                .and_then(Value::as_i64)
                .filter(|n| (min..=max).contains(n))
                .unwrap_or(fallback)
        };
        let bool_of = |key: &str, fallback: bool| {
            settings.get(key).and_then(Value::as_bool).unwrap_or(fallback)
        };
        Self {
            max_post_length:           int_in("max_post_length", 100, 1_000_000, d.max_post_length as i64) as usize,
            post_edit_window_minutes:  int_in("post_edit_window_minutes", 0, 10_080, d.post_edit_window_minutes),
            min_seconds_between_posts: int_in("min_seconds_between_posts", 0, 3_600, d.min_seconds_between_posts),
            post_approval_mode:        ApprovalMode::parse(
                settings.get("post_approval_mode").and_then(Value::as_str),
                d.post_approval_mode,
            ),
            new_member_post_count:     int_in("new_member_post_count", 1, 10_000, d.new_member_post_count as i64) as i32,
            pending_retention_days:    int_in("pending_retention_days", 0, 365, d.pending_retention_days as i64) as i32,
            allow_signatures:          bool_of("allow_signatures", d.allow_signatures),
            max_signature_length:      int_in("max_signature_length", 0, 2_000, d.max_signature_length as i64) as usize,
        }
    }
}

/// Reads the instance settings from the core. Any failure yields `None`, so the
/// caller keeps the values it already had rather than reverting to defaults
/// because the core was briefly unreachable.
pub async fn fetch(http: &reqwest::Client, core_url: &str, secret: &str) -> Option<InstanceConfig> {
    let url = format!("{core_url}/internal/modules/forum/settings");
    let resp = http
        .get(&url)
        .header("X-Internal-Secret", secret)
        .send()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Reading forum instance settings"))
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "Forum instance settings refused by the core");
        return None;
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Forum instance settings: unreadable response"))
        .ok()?;

    Some(InstanceConfig::from_settings(body.get("settings")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_keys_keep_the_compiled_defaults() {
        let c = InstanceConfig::from_settings(&json!({}));
        assert_eq!(c.max_post_length, 100_000);
        assert_eq!(c.post_edit_window_minutes, 0);
        assert_eq!(c.min_seconds_between_posts, 0);
        assert_eq!(c.post_approval_mode, ApprovalMode::None);
        assert_eq!(c.new_member_post_count, 5);
        assert_eq!(c.pending_retention_days, 14);
        assert!(c.allow_signatures);
    }

    #[test]
    fn an_unknown_approval_mode_never_disarms_moderation() {
        // Start from a moderated instance, then feed it nonsense: the fallback is
        // the value passed in, not the permissive one.
        let c = InstanceConfig::from_settings(&json!({ "post_approval_mode": "sometimes" }));
        assert_eq!(c.post_approval_mode, ApprovalMode::None); // compiled default
        assert_eq!(
            ApprovalMode::parse(Some("nonsense"), ApprovalMode::All),
            ApprovalMode::All
        );
    }

    #[test]
    fn moderators_are_never_queued() {
        for mode in [ApprovalMode::None, ApprovalMode::NewMembers, ApprovalMode::All] {
            assert!(mode.publishes_immediately(true, 0, 5));
        }
    }

    #[test]
    fn new_member_mode_only_queues_below_the_threshold() {
        let m = ApprovalMode::NewMembers;
        assert!(!m.publishes_immediately(false, 0, 5));
        assert!(!m.publishes_immediately(false, 4, 5));
        assert!(m.publishes_immediately(false, 5, 5));
        assert!(m.publishes_immediately(false, 900, 5));
    }

    #[test]
    fn all_mode_queues_every_regular_member() {
        assert!(!ApprovalMode::All.publishes_immediately(false, 10_000, 5));
        assert!(ApprovalMode::All.is_active());
        assert!(!ApprovalMode::None.is_active());
    }

    #[test]
    fn out_of_range_values_fall_back() {
        let c = InstanceConfig::from_settings(&json!({
            "max_post_length": 4, "min_seconds_between_posts": 999_999,
        }));
        assert_eq!(c.max_post_length, 100_000);
        assert_eq!(c.min_seconds_between_posts, 0);
    }
}
