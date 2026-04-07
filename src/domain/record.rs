use chrono::{DateTime, FixedOffset};

use crate::domain::submission::SubmissionRecord;

#[derive(Debug, Clone)]
pub enum SyncSelection {
    Submission(SubmissionRecord),
    Chore,
    Delete,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveRecord {
    pub problem_id: String,
    pub title: String,
    pub verdict: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
    pub file_name: String,
    pub source_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalSolveRecord {
    pub revision: String,
    pub record: SolveRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecordSummary {
    pub revision: String,
    pub problem_id: String,
    pub title: String,
    pub file_name: String,
    pub verdict: String,
    pub difficulty: String,
    pub submission_id: Option<u64>,
    pub submission_time: Option<DateTime<FixedOffset>>,
}
