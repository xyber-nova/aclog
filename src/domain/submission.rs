use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::problem::ProblemProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionRecord {
    pub submission_id: u64,
    pub problem_id: Option<String>,
    #[serde(default)]
    pub provider: ProblemProvider,
    pub submitter: String,
    pub verdict: String,
    pub score: Option<i64>,
    pub time_ms: Option<u64>,
    pub memory_mb: Option<f64>,
    pub submitted_at: Option<DateTime<FixedOffset>>,
}
