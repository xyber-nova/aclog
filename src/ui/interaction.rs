//! 应用层交互抽象。
//!
//! 这一层只定义 workflow 需要向“界面”请求什么能力，
//! 不关心能力最终由真实 TUI、测试替身还是别的前端实现。

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

#[derive(Debug, Clone)]
pub enum SyncBatchReviewAction {
    /// 暂停当前批次，让 app workflow 保留恢复状态并退出交互界面。
    Pause,
    /// 仅打开某个批次项的详情页，不在预览页直接做决策。
    Open(usize),
    /// 在预览页直接做出一个安全决策，由 app workflow 继续落到真实处理步骤。
    Decide {
        index: usize,
        selection: SyncSelection,
    },
}

#[derive(Debug, Clone)]
pub enum SyncBatchDetailAction {
    /// 从详情页返回上一层，不隐含任何业务决策。
    Back,
    /// 详情页已经拿到一个明确决策，交给 workflow 后续执行。
    Decide(SyncSelection),
    /// 直接退出当前 sync 工作台，并保留恢复状态。
    Quit,
}

/// CLI workflow 可依赖的交互能力集合。
///
/// 这里的边界是：
/// - 负责把已经准备好的候选集合展示给用户，并返回选择结果
/// - 不负责访问 API、读取仓库、修改仓库、或隐式补齐业务判断
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

    fn review_sync_batch_action(
        &self,
        workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<SyncBatchReviewAction> {
        // 老接口只返回“打开哪一项或暂停”，
        // 默认实现把它提升成新的动作枚举，保证 fake UI 不需要立刻大改。
        self.review_sync_batch(workspace_root, session)
            .map(|choice| choice.map_or(SyncBatchReviewAction::Pause, SyncBatchReviewAction::Open))
    }

    fn select_sync_batch_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection>;

    fn select_sync_batch_detail_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncBatchDetailAction> {
        // 老接口只关心最终 selection；
        // 默认实现把它包装成“详情页给出决策”的语义。
        self.select_sync_batch_action(item, metadata, submissions)
            .map(SyncBatchDetailAction::Decide)
    }

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
        crate::ui::terminal::review_sync_batch(workspace_root, session)
    }

    fn review_sync_batch_action(
        &self,
        workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<SyncBatchReviewAction> {
        crate::ui::terminal::review_sync_batch_action(workspace_root, session)
    }

    fn select_sync_batch_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        crate::ui::terminal::select_sync_batch_action(item, metadata, submissions)
    }

    fn select_sync_batch_detail_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncBatchDetailAction> {
        crate::ui::terminal::select_sync_batch_detail_action(item, metadata, submissions)
    }

    fn select_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        crate::ui::terminal::select_submission(problem_id, metadata, submissions)
    }

    fn select_record_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<Option<SubmissionRecord>> {
        crate::ui::terminal::select_record_submission(problem_id, metadata, submissions)
    }

    fn select_record_to_rebind(
        &self,
        problem_id: &str,
        file_name: &str,
        records: &[HistoricalSolveRecord],
    ) -> Result<Option<HistoricalSolveRecord>> {
        crate::ui::terminal::select_record_to_rebind(problem_id, file_name, records)
    }

    fn confirm_deleted_file(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
    ) -> Result<SyncSelection> {
        crate::ui::terminal::confirm_deleted_file(problem_id, metadata)
    }

    fn open_record_browser(
        &self,
        workspace_root: &Path,
        query: &BrowserQuery,
        index: &RecordIndex,
    ) -> Result<()> {
        crate::ui::terminal::open_record_browser(workspace_root, query, index)
    }

    fn show_stats_dashboard(
        &self,
        workspace_root: &Path,
        dashboard: &StatsDashboard,
        index: &RecordIndex,
    ) -> Result<()> {
        crate::ui::terminal::show_stats_dashboard(workspace_root, dashboard, index)
    }

    fn show_stats(&self, workspace_root: &Path, summary: &StatsSummary) -> Result<()> {
        crate::ui::terminal::show_stats(workspace_root, summary)
    }
}
