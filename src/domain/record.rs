use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::{domain::submission::SubmissionRecord, problem::ProblemProvider};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingFields {
    pub note: Option<String>,
    pub mistakes: Option<String>,
    pub insight: Option<String>,
    pub confidence: Option<String>,
    pub source_kind: Option<String>,
    pub time_spent: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SyncSelection {
    Submission(SubmissionRecord),
    Chore,
    Delete,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveRecord {
    pub problem_id: String,
    #[serde(default)]
    pub provider: ProblemProvider,
    pub title: String,
    pub verdict: String,
    pub score: Option<i64>,
    pub time_ms: Option<u64>,
    pub memory_mb: Option<f64>,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub source: String,
    #[serde(default)]
    pub contest: Option<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub file_name: String,
    pub training: TrainingFields,
    pub source_order: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalSolveRecord {
    pub revision: String,
    pub record: SolveRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRecordSummary {
    pub revision: String,
    pub problem_id: String,
    pub provider: ProblemProvider,
    pub title: String,
    pub file_name: String,
    pub verdict: String,
    pub score: Option<i64>,
    pub time_ms: Option<u64>,
    pub memory_mb: Option<f64>,
    pub difficulty: String,
    pub source: String,
    pub contest: Option<String>,
    pub tags: Vec<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub training: TrainingFields,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemRecordSummary {
    pub problem_id: String,
    pub provider: ProblemProvider,
    pub title: String,
    pub verdict: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub source: String,
    pub contest: Option<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub files: Vec<String>,
    pub latest_revision: String,
}
