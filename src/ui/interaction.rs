use std::path::Path;

use color_eyre::Result;

use crate::domain::{
    browser::BrowserQuery,
    problem::ProblemMetadata,
    record::{HistoricalSolveRecord, SyncSelection},
    record_index::RecordIndex,
    stats::{StatsDashboard, StatsSummary},
    submission::SubmissionRecord,
    sync_batch::{SyncBatchSession, SyncSessionChoice, SyncSessionItem},
};

pub trait UserInterface {
    fn choose_sync_session_action(
        &self,
        workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<SyncSessionChoice>;

    fn review_sync_batch(
        &self,
        workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<Option<usize>>;

    fn select_sync_batch_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection>;

    fn select_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection>;

    fn select_record_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<Option<SubmissionRecord>>;

    fn select_record_to_rebind(
        &self,
        problem_id: &str,
        file_name: &str,
        records: &[HistoricalSolveRecord],
    ) -> Result<Option<HistoricalSolveRecord>>;

    fn confirm_deleted_file(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
    ) -> Result<SyncSelection>;

    fn open_record_browser(
        &self,
        workspace_root: &Path,
        query: &BrowserQuery,
        index: &RecordIndex,
    ) -> Result<()>;

    fn show_stats_dashboard(
        &self,
        workspace_root: &Path,
        dashboard: &StatsDashboard,
        index: &RecordIndex,
    ) -> Result<()>;

    fn show_stats(&self, workspace_root: &Path, summary: &StatsSummary) -> Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalUi;

impl UserInterface for TerminalUi {
    fn choose_sync_session_action(
        &self,
        workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<SyncSessionChoice> {
        crate::tui::choose_sync_session_action(workspace_root, session)
    }

    fn review_sync_batch(
        &self,
        workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<Option<usize>> {
        crate::tui::review_sync_batch(workspace_root, session)
    }

    fn select_sync_batch_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        crate::tui::select_sync_batch_action(item, metadata, submissions)
    }

    fn select_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        crate::tui::select_submission(problem_id, metadata, submissions)
    }

    fn select_record_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<Option<SubmissionRecord>> {
        crate::tui::select_record_submission(problem_id, metadata, submissions)
    }

    fn select_record_to_rebind(
        &self,
        problem_id: &str,
        file_name: &str,
        records: &[HistoricalSolveRecord],
    ) -> Result<Option<HistoricalSolveRecord>> {
        crate::tui::select_record_to_rebind(problem_id, file_name, records)
    }

    fn confirm_deleted_file(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
    ) -> Result<SyncSelection> {
        crate::tui::confirm_deleted_file(problem_id, metadata)
    }

    fn open_record_browser(
        &self,
        workspace_root: &Path,
        query: &BrowserQuery,
        index: &RecordIndex,
    ) -> Result<()> {
        crate::tui::open_record_browser(workspace_root, query, index)
    }

    fn show_stats_dashboard(
        &self,
        workspace_root: &Path,
        dashboard: &StatsDashboard,
        index: &RecordIndex,
    ) -> Result<()> {
        crate::tui::show_stats_dashboard(workspace_root, dashboard, index)
    }

    fn show_stats(&self, workspace_root: &Path, summary: &StatsSummary) -> Result<()> {
        crate::tui::show_stats(workspace_root, summary)
    }
}
