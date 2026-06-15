use crate::state::AppState;
use serde_json::json;
use uuid::Uuid;

pub async fn publish_topic_created(state: &AppState, topic_id: Uuid, user_id: Uuid) {
    publish(state, "ForumTopicCreated", "topic_id", topic_id, user_id).await;
}

pub async fn publish_post_created(state: &AppState, post_id: Uuid, user_id: Uuid) {
    publish(state, "ForumPostCreated", "post_id", post_id, user_id).await;
}

pub async fn publish_post_updated(state: &AppState, post_id: Uuid, user_id: Uuid) {
    publish(state, "ForumPostUpdated", "post_id", post_id, user_id).await;
}

pub async fn publish_post_deleted(state: &AppState, post_id: Uuid, user_id: Uuid) {
    publish(state, "ForumPostDeleted", "post_id", post_id, user_id).await;
}

pub async fn publish_reported(state: &AppState, post_id: Uuid, user_id: Uuid) {
    publish(state, "ForumReported", "post_id", post_id, user_id).await;
}

async fn publish(state: &AppState, event_type: &str, id_key: &str, id: Uuid, user_id: Uuid) {
    let payload = json!({
        "type": event_type,
        "payload": {
            id_key:      id,
            "user_id":   user_id,
            "module_id": "forum",
        }
    });
    send_to_core(state, &payload).await;
}

async fn send_to_core(state: &AppState, payload: &serde_json::Value) {
    let url = format!("{}/internal/events/publish", state.settings.core.url);
    match reqwest::Client::new()
        .post(&url)
        .header("X-Internal-Secret", &state.settings.core.internal_secret)
        .json(payload)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => tracing::warn!(status = %r.status(), "Publish event: unexpected response"),
        Err(e) => tracing::warn!(error = %e, "Publish event: network error"),
    }
}
