use std::sync::LazyLock;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

/// How long a resolved display name stays valid in the cache. Short enough
/// that a rename shows up quickly, long enough to spare the core's directory
/// endpoint from a burst of lookups (e.g. one reply notifying many watchers).
const TTL: Duration = Duration::from_secs(60);

/// How long we wait for the core's directory endpoint before giving up and
/// falling back to a generic title. Naming the actor is a nice-to-have, never
/// a reason to delay or fail a notification.
const TIMEOUT: Duration = Duration::from_millis(800);

static NAME_CACHE: LazyLock<DashMap<Uuid, (String, Instant)>> = LazyLock::new(DashMap::new);

#[derive(Deserialize)]
struct DirectoryUser {
    display_name: Option<String>,
    username:     Option<String>,
}

/// Best-effort resolution of a user's public display name through the core's
/// internal directory, with a short-lived cache so notifying N recipients for
/// the same actor costs at most one HTTP round trip per TTL window. Returns
/// `None` on any cache miss + network/parse failure — callers must fall back
/// to a generic, name-less title in that case, never fail on account of this.
pub async fn display_name(state: &AppState, user_id: Uuid) -> Option<String> {
    if let Some(entry) = NAME_CACHE.get(&user_id) {
        let (name, fetched_at) = entry.value();
        if fetched_at.elapsed() < TTL {
            return Some(name.clone());
        }
    }

    let url = format!("{}/internal/directory/users/{}", state.settings.core.url, user_id);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("X-Internal-Secret", &state.settings.core.internal_secret)
        .timeout(TIMEOUT)
        .send()
        .await;

    let user = match resp {
        Ok(r) if r.status().is_success() => match r.json::<DirectoryUser>().await {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(error = %e, "forum directory lookup: bad response body");
                return None;
            }
        },
        Ok(r) => {
            tracing::warn!(status = %r.status(), "forum directory lookup: unexpected response");
            return None;
        }
        Err(e) => {
            tracing::warn!(error = %e, "forum directory lookup: network error");
            return None;
        }
    };

    let name = user.display_name.filter(|n| !n.is_empty()).or(user.username)?;
    NAME_CACHE.insert(user_id, (name.clone(), Instant::now()));
    Some(name)
}

/// A core user group, id + name only (the core never discloses a group's
/// permission policy to a module).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryGroup {
    pub id:   Uuid,
    pub name: String,
}

#[derive(Deserialize)]
struct GroupsResponse {
    groups: Vec<DirectoryGroup>,
}

static GROUPS_CACHE: LazyLock<DashMap<Uuid, (Vec<Uuid>, Instant)>> = LazyLock::new(DashMap::new);

/// The core user-group ids a member belongs to, resolved through the core's
/// internal directory with the same short-lived cache as [`display_name`].
///
/// SECURITY: this only ever GRANTS additional forum access (see
/// `permission_service`), so it fails CLOSED — any network/parse error, or an
/// unknown user, yields an EMPTY set, never a guessed one. A core outage can
/// therefore only withhold group-granted extras, never widen visibility, and
/// the role-based rules keep working untouched.
pub async fn user_group_ids(state: &AppState, user_id: Uuid) -> Vec<Uuid> {
    if let Some(entry) = GROUPS_CACHE.get(&user_id) {
        let (ids, fetched_at) = entry.value();
        if fetched_at.elapsed() < TTL {
            return ids.clone();
        }
    }

    let url = format!("{}/internal/directory/users/{}/groups", state.settings.core.url, user_id);
    let ids = match reqwest::Client::new()
        .get(&url)
        .header("X-Internal-Secret", &state.settings.core.internal_secret)
        .timeout(TIMEOUT)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json::<GroupsResponse>().await {
            Ok(g) => g.groups.into_iter().map(|g| g.id).collect(),
            Err(e) => {
                tracing::warn!(error = %e, "forum group lookup: bad response body");
                Vec::new()
            }
        },
        Ok(r) => {
            tracing::warn!(status = %r.status(), "forum group lookup: unexpected response");
            Vec::new()
        }
        Err(e) => {
            tracing::warn!(error = %e, "forum group lookup: network error");
            Vec::new()
        }
    };

    GROUPS_CACHE.insert(user_id, (ids.clone(), Instant::now()));
    ids
}

/// Every group of the instance (id + name), for an admin picking which groups a
/// per-group forum rule applies to. Empty on any failure — the admin UI then
/// simply shows no groups rather than erroring.
pub async fn list_groups(state: &AppState) -> Vec<DirectoryGroup> {
    let url = format!("{}/internal/directory/groups", state.settings.core.url);
    match reqwest::Client::new()
        .get(&url)
        .header("X-Internal-Secret", &state.settings.core.internal_secret)
        .timeout(TIMEOUT)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            r.json::<GroupsResponse>().await.map(|g| g.groups).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}
