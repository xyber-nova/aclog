//! Compatibility facade for the legacy `crate::tui` module path.
//!
//! New terminal UI code lives under `crate::ui::terminal`.

use std::path::Path;

use color_eyre::Result;

use crate::models::{
    BrowserQuery, HistoricalSolveRecord, ProblemMetadata, RecordIndex, StatsDashboard,
    StatsSummary, SubmissionRecord, SyncBatchSession, SyncSelection, SyncSessionChoice,
    SyncSessionItem,
};

pub fn select_submission(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    crate::ui::terminal::select_submission(problem_id, metadata, submissions)
}

pub fn select_record_submission(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<Option<SubmissionRecord>> {
    crate::ui::terminal::select_record_submission(problem_id, metadata, submissions)
}

pub fn select_record_to_rebind(
    problem_id: &str,
    file_name: &str,
    records: &[HistoricalSolveRecord],
) -> Result<Option<HistoricalSolveRecord>> {
    crate::ui::terminal::select_record_to_rebind(problem_id, file_name, records)
}

pub fn confirm_deleted_file(
    problem_id: &str,
    metadata: Option<&ProblemMetadata>,
) -> Result<SyncSelection> {
    crate::ui::terminal::confirm_deleted_file(problem_id, metadata)
}

pub fn show_stats(workspace_root: &Path, summary: &StatsSummary) -> Result<()> {
    crate::ui::terminal::show_stats(workspace_root, summary)
}

pub fn choose_sync_session_action(
    workspace_root: &Path,
    session: &SyncBatchSession,
) -> Result<SyncSessionChoice> {
    crate::ui::terminal::choose_sync_session_action(workspace_root, session)
}

pub fn review_sync_batch(
    workspace_root: &Path,
    session: &SyncBatchSession,
) -> Result<Option<usize>> {
    crate::ui::terminal::review_sync_batch(workspace_root, session)
}

pub fn select_sync_batch_action(
    item: &SyncSessionItem,
    metadata: Option<&ProblemMetadata>,
    submissions: &[SubmissionRecord],
) -> Result<SyncSelection> {
    crate::ui::terminal::select_sync_batch_action(item, metadata, submissions)
}

pub fn open_record_browser(
    workspace_root: &Path,
    query: &BrowserQuery,
    index: &RecordIndex,
) -> Result<()> {
    crate::ui::terminal::open_record_browser(workspace_root, query, index)
}

pub fn show_stats_dashboard(
    workspace_root: &Path,
    dashboard: &StatsDashboard,
    index: &RecordIndex,
) -> Result<()> {
    crate::ui::terminal::show_stats_dashboard(workspace_root, dashboard, index)
}
