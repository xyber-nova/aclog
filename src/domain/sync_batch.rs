use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncSessionChoice {
    Resume,
    Rebuild,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SyncChangeKind {
    Active,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncItemStatus {
    Pending,
    Planned,
    Skipped,
    Committed,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStoredDecision {
    Submission { submission_id: u64 },
    Chore,
    Delete,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncWarningCode {
    DuplicateSubmission,
    SubmissionProblemMismatch,
    InvalidatedByWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncWarning {
    pub code: SyncWarningCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncSessionItem {
    pub file: String,
    pub problem_id: Option<String>,
    pub kind: SyncChangeKind,
    pub status: SyncItemStatus,
    pub submissions: Option<usize>,
    pub default_submission_id: Option<u64>,
    pub decision: Option<SyncStoredDecision>,
    #[serde(default)]
    pub warnings: Vec<SyncWarning>,
    pub invalid_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncBatchSession {
    pub created_at: DateTime<FixedOffset>,
    pub items: Vec<SyncSessionItem>,
}

impl SyncSessionItem {
    pub fn is_pending(&self) -> bool {
        matches!(self.status, SyncItemStatus::Pending)
    }

    pub fn is_decided(&self) -> bool {
        matches!(
            self.status,
            SyncItemStatus::Planned | SyncItemStatus::Skipped | SyncItemStatus::Committed
        )
    }
}
