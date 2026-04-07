use std::path::Path;

use color_eyre::Result;

use crate::domain::{
    problem::ProblemMetadata,
    record::{HistoricalSolveRecord, SyncSelection},
    stats::StatsSummary,
    submission::SubmissionRecord,
};

pub trait UserInterface {
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

    fn show_stats(&self, workspace_root: &Path, summary: &StatsSummary) -> Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalUi;

impl UserInterface for TerminalUi {
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

    fn show_stats(&self, workspace_root: &Path, summary: &StatsSummary) -> Result<()> {
        crate::tui::show_stats(workspace_root, summary)
    }
}
