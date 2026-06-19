use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Poll {
    pub id:          Uuid,
    pub topic_id:    Uuid,
    pub question:    String,
    pub is_multiple: bool,
    pub closes_at:   Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PollOptionResult {
    pub id:    Uuid,
    pub text:  String,
    pub votes: i64,
    pub me:    bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PollResults {
    pub poll:         Poll,
    pub options:      Vec<PollOptionResult>,
    pub total_voters: i64,
    pub has_voted:    bool,
    pub is_closed:    bool,
}

#[derive(Debug, Deserialize)]
pub struct VoteDto {
    pub option_ids: Vec<Uuid>,
}
