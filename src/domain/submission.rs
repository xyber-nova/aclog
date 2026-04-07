use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionRecord {
    pub submission_id: u64,
    pub submitter: String,
    pub verdict: String,
    pub score: Option<i64>,
    pub time_ms: Option<u64>,
    pub memory_mb: Option<f64>,
    pub submitted_at: Option<DateTime<FixedOffset>>,
}
