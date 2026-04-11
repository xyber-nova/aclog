#![allow(dead_code)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use aclog::{
    app::deps::{JjRepository, OutputSink, ProblemProvider},
    config::{AclogPaths, AppConfig},
    domain::{
        browser::BrowserQuery,
        problem::ProblemMetadata,
        record::{HistoricalSolveRecord, SolveRecord, SyncSelection, TrainingFields},
        record_index::RecordIndex,
        stats::{StatsDashboard, StatsSummary},
        submission::SubmissionRecord,
        sync_batch::{SyncBatchSession, SyncSessionChoice, SyncSessionItem},
    },
    ui::interaction::{HomeAction, HomeSummary, SyncBatchDetailAction, UserInterface},
    vcs::{ProblemFileChange, ProblemFileChangeKind},
};
use chrono::{FixedOffset, TimeZone};
use color_eyre::{Result, eyre::eyre};
use tempfile::TempDir;

#[derive(Default)]
pub struct FakeDeps {
    changed_files: Mutex<Vec<ProblemFileChange>>,
    metadata_by_problem: Mutex<HashMap<String, Option<ProblemMetadata>>>,
    submissions_by_problem: Mutex<HashMap<String, Vec<SubmissionRecord>>>,
    submission_fetch_counts: Mutex<HashMap<String, usize>>,
    submission_fetch_barrier: Mutex<Option<Arc<tokio::sync::Barrier>>>,
    submission_fetch_in_flight: AtomicUsize,
    submission_fetch_max_in_flight: AtomicUsize,
    algorithm_tag_names: Mutex<HashSet<String>>,
    commit_descriptions: Mutex<Vec<(String, String)>>,
    resolved_revsets: Mutex<HashMap<String, String>>,
    tracked_files: Mutex<HashSet<String>>,
    created_commits: Mutex<Vec<(String, String)>>,
    rewritten_descriptions: Mutex<Vec<(String, String)>>,
    outputs: Mutex<Vec<String>>,
}

impl FakeDeps {
    pub fn set_changed_files(&self, files: Vec<ProblemFileChange>) {
        *self.changed_files.lock().unwrap() = files;
    }

    pub fn insert_metadata(&self, problem_id: &str, metadata: Option<ProblemMetadata>) {
        self.metadata_by_problem
            .lock()
            .unwrap()
            .insert(problem_id.to_string(), metadata);
    }

    pub fn insert_submissions(&self, problem_id: &str, submissions: Vec<SubmissionRecord>) {
        self.submissions_by_problem
            .lock()
            .unwrap()
            .insert(problem_id.to_string(), submissions);
    }

    pub fn set_algorithm_tag_names(&self, names: &[&str]) {
        *self.algorithm_tag_names.lock().unwrap() =
            names.iter().map(|name| (*name).to_string()).collect();
    }

    pub fn configure_submission_fetch_barrier(&self, parties: usize) {
        *self.submission_fetch_barrier.lock().unwrap() =
            Some(Arc::new(tokio::sync::Barrier::new(parties)));
    }

    pub fn set_commit_descriptions(&self, entries: Vec<(String, String)>) {
        *self.commit_descriptions.lock().unwrap() = entries;
    }

    pub fn resolve_revset_as(&self, revset: &str, revision: &str) {
        self.resolved_revsets
            .lock()
            .unwrap()
            .insert(revset.to_string(), revision.to_string());
    }

    pub fn track_file(&self, file: &str) {
        self.tracked_files.lock().unwrap().insert(file.to_string());
    }

    pub fn created_commits(&self) -> Vec<(String, String)> {
        self.created_commits.lock().unwrap().clone()
    }

    pub fn rewritten_descriptions(&self) -> Vec<(String, String)> {
        self.rewritten_descriptions.lock().unwrap().clone()
    }

    pub fn outputs(&self) -> Vec<String> {
        self.outputs.lock().unwrap().clone()
    }

    pub fn submission_fetch_count(&self, problem_id: &str) -> usize {
        self.submission_fetch_counts
            .lock()
            .unwrap()
            .get(problem_id)
            .copied()
            .unwrap_or_default()
    }

    pub fn max_submission_fetch_in_flight(&self) -> usize {
        self.submission_fetch_max_in_flight.load(Ordering::SeqCst)
    }
}

impl ProblemProvider for FakeDeps {
    async fn resolve_problem_metadata(
        &self,
        _config: &AppConfig,
        _paths: &AclogPaths,
        problem_id: &str,
    ) -> Result<Option<ProblemMetadata>> {
        Ok(self
            .metadata_by_problem
            .lock()
            .unwrap()
            .get(problem_id)
            .cloned()
            .unwrap_or(None))
    }

    async fn fetch_problem_submissions(
        &self,
        _config: &AppConfig,
        _paths: &AclogPaths,
        problem_id: &str,
    ) -> Result<Vec<SubmissionRecord>> {
        {
            let mut counts = self.submission_fetch_counts.lock().unwrap();
            *counts.entry(problem_id.to_string()).or_default() += 1;
        }
        let in_flight = self
            .submission_fetch_in_flight
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        let mut observed_max = self.submission_fetch_max_in_flight.load(Ordering::SeqCst);
        while in_flight > observed_max {
            match self.submission_fetch_max_in_flight.compare_exchange(
                observed_max,
                in_flight,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(current) => observed_max = current,
            }
        }
        let barrier = self.submission_fetch_barrier.lock().unwrap().clone();
        if let Some(barrier) = barrier {
            barrier.wait().await;
        } else {
            tokio::task::yield_now().await;
        }
        self.submission_fetch_in_flight
            .fetch_sub(1, Ordering::SeqCst);
        Ok(self
            .submissions_by_problem
            .lock()
            .unwrap()
            .get(problem_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn load_algorithm_tag_names(
        &self,
        _config: &AppConfig,
        _paths: &AclogPaths,
        _provider: aclog::problem::ProblemProvider,
    ) -> Result<HashSet<String>> {
        Ok(self.algorithm_tag_names.lock().unwrap().clone())
    }
}

impl JjRepository for FakeDeps {
    async fn ensure_workspace(&self) -> Result<()> {
        Ok(())
    }

    async fn detect_working_copy_changes(&self) -> Result<Vec<ProblemFileChange>> {
        Ok(self.changed_files.lock().unwrap().clone())
    }

    async fn load_record_index(&self) -> Result<RecordIndex> {
        let entries = self.commit_descriptions.lock().unwrap().clone();
        let records = aclog::commit_format::parse_historical_solve_records(&entries);
        Ok(RecordIndex::build(&records))
    }

    async fn resolve_revision(&self, revset_str: &str) -> Result<String> {
        self.resolved_revsets
            .lock()
            .unwrap()
            .get(revset_str)
            .cloned()
            .ok_or_else(|| eyre!("missing fake revision for `{revset_str}`"))
    }

    async fn is_tracked_file(&self, repo_relative_path: &str) -> Result<bool> {
        Ok(self
            .tracked_files
            .lock()
            .unwrap()
            .contains(repo_relative_path))
    }

    async fn create_commits(&self, commits: &[(String, String)]) -> Result<()> {
        self.created_commits
            .lock()
            .unwrap()
            .extend(commits.iter().cloned());
        let mut entries = self.commit_descriptions.lock().unwrap();
        let base = entries.len();
        for (index, (_file, message)) in commits.iter().enumerate() {
            entries.push((format!("fake-created-{}", base + index), message.clone()));
        }
        Ok(())
    }

    async fn rewrite_commit_description(&self, revision: &str, message: &str) -> Result<()> {
        self.rewritten_descriptions
            .lock()
            .unwrap()
            .push((revision.to_string(), message.to_string()));
        let mut entries = self.commit_descriptions.lock().unwrap();
        if let Some(entry) = entries
            .iter_mut()
            .find(|(candidate, _)| candidate == revision)
        {
            entry.1 = message.to_string();
        }
        Ok(())
    }
}

impl OutputSink for FakeDeps {
    fn write_output(&self, text: &str) -> Result<()> {
        self.outputs.lock().unwrap().push(text.to_string());
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeUi {
    pub home_actions: Mutex<Vec<HomeAction>>,
    pub shown_home_summaries: Mutex<Vec<HomeSummary>>,
    pub sync_session_choice: Mutex<Option<SyncSessionChoice>>,
    pub sync_batch_review_selection: Mutex<Vec<Option<usize>>>,
    pub sync_batch_detail_action: Mutex<Option<SyncBatchDetailAction>>,
    pub submission_selection: Mutex<Option<SyncSelection>>,
    pub record_submission_selection: Mutex<Option<Option<SubmissionRecord>>>,
    pub record_to_rebind_selection: Mutex<Option<Option<HistoricalSolveRecord>>>,
    pub delete_confirmation: Mutex<Option<SyncSelection>>,
    pub submission_requests: Mutex<Vec<(String, Option<ProblemMetadata>, Vec<SubmissionRecord>)>>,
    pub record_submission_requests:
        Mutex<Vec<(String, Option<ProblemMetadata>, Vec<SubmissionRecord>)>>,
    pub shown_stats: Mutex<Vec<StatsSummary>>,
    pub shown_dashboards: Mutex<Vec<StatsDashboard>>,
    pub opened_browsers: Mutex<Vec<BrowserQuery>>,
}

impl FakeUi {
    pub fn with_home_actions(actions: Vec<HomeAction>) -> Self {
        Self {
            home_actions: Mutex::new(actions),
            ..Self::default()
        }
    }

    pub fn with_submission_selection(selection: SyncSelection) -> Self {
        Self {
            submission_selection: Mutex::new(Some(selection)),
            ..Self::default()
        }
    }

    pub fn with_record_submission(record: Option<SubmissionRecord>) -> Self {
        Self {
            record_submission_selection: Mutex::new(Some(record)),
            ..Self::default()
        }
    }

    pub fn with_record_to_rebind(record: Option<HistoricalSolveRecord>) -> Self {
        Self {
            record_to_rebind_selection: Mutex::new(Some(record)),
            ..Self::default()
        }
    }

    pub fn with_delete_confirmation(selection: SyncSelection) -> Self {
        Self {
            delete_confirmation: Mutex::new(Some(selection)),
            ..Self::default()
        }
    }
}

impl UserInterface for FakeUi {
    fn open_home(&self, _workspace_root: &Path, summary: &HomeSummary) -> Result<HomeAction> {
        self.shown_home_summaries
            .lock()
            .unwrap()
            .push(summary.clone());
        let mut actions = self.home_actions.lock().unwrap();
        if actions.is_empty() {
            return Ok(HomeAction::Exit);
        }
        Ok(actions.remove(0))
    }

    fn choose_sync_session_action(
        &self,
        _workspace_root: &Path,
        _session: &SyncBatchSession,
    ) -> Result<SyncSessionChoice> {
        self.sync_session_choice
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected choose_sync_session_action call"))
    }

    fn review_sync_batch(
        &self,
        _workspace_root: &Path,
        session: &SyncBatchSession,
    ) -> Result<Option<usize>> {
        let mut selections = self.sync_batch_review_selection.lock().unwrap();
        if selections.is_empty() {
            return Ok(session.items.iter().position(|item| {
                item.status == aclog::domain::sync_batch::SyncItemStatus::Pending
            }));
        }
        Ok(selections.remove(0))
    }

    fn select_sync_batch_action(
        &self,
        item: &SyncSessionItem,
        _metadata: Option<&ProblemMetadata>,
        _submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        if matches!(
            item.kind,
            aclog::domain::sync_batch::SyncChangeKind::Deleted
        ) {
            return self
                .delete_confirmation
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| eyre!("unexpected select_sync_batch_action delete call"));
        }
        self.submission_selection
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected select_sync_batch_action call"))
    }

    fn select_sync_batch_detail_action(
        &self,
        item: &SyncSessionItem,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncBatchDetailAction> {
        if let Some(action) = self.sync_batch_detail_action.lock().unwrap().clone() {
            return Ok(action);
        }
        self.select_sync_batch_action(item, metadata, submissions)
            .map(SyncBatchDetailAction::Decide)
    }

    fn select_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        self.submission_requests.lock().unwrap().push((
            problem_id.to_string(),
            metadata.cloned(),
            submissions.to_vec(),
        ));
        self.submission_selection
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected select_submission call"))
    }

    fn select_record_submission(
        &self,
        problem_id: &str,
        metadata: Option<&ProblemMetadata>,
        submissions: &[SubmissionRecord],
    ) -> Result<Option<SubmissionRecord>> {
        self.record_submission_requests.lock().unwrap().push((
            problem_id.to_string(),
            metadata.cloned(),
            submissions.to_vec(),
        ));
        self.record_submission_selection
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected select_record_submission call"))
    }

    fn select_record_to_rebind(
        &self,
        _problem_id: &str,
        _file_name: &str,
        _records: &[HistoricalSolveRecord],
    ) -> Result<Option<HistoricalSolveRecord>> {
        self.record_to_rebind_selection
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected select_record_to_rebind call"))
    }

    fn confirm_deleted_file(
        &self,
        _problem_id: &str,
        _metadata: Option<&ProblemMetadata>,
    ) -> Result<SyncSelection> {
        self.delete_confirmation
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected confirm_deleted_file call"))
    }

    fn open_record_browser(
        &self,
        _workspace_root: &Path,
        query: &BrowserQuery,
        _index: &RecordIndex,
    ) -> Result<()> {
        self.opened_browsers.lock().unwrap().push(query.clone());
        Ok(())
    }

    fn show_stats_dashboard(
        &self,
        _workspace_root: &Path,
        dashboard: &StatsDashboard,
        _index: &RecordIndex,
    ) -> Result<()> {
        self.shown_dashboards
            .lock()
            .unwrap()
            .push(dashboard.clone());
        Ok(())
    }

    fn show_stats(&self, _workspace_root: &Path, summary: &StatsSummary) -> Result<()> {
        self.shown_stats.lock().unwrap().push(summary.clone());
        Ok(())
    }
}

pub fn workspace_with_config() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let aclog_dir = dir.path().join(".aclog");
    fs::create_dir_all(aclog_dir.join("problems")).unwrap();
    fs::write(
        aclog_dir.join("config.toml"),
        "[user]\nluogu_uid = \"123\"\nluogu_cookie = \"cookie\"\n\n[settings]\nmetadata_ttl_days = 7\nproblem_metadata_ttl_days = 7\nluogu_mappings_ttl_days = 7\nluogu_tags_ttl_days = 7\nreview_problem_interval_days = 21\npractice_tag_window_days = 60\npractice_tag_target_problems = 5\n",
    )
    .unwrap();
    dir
}

pub async fn init_real_workspace() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    aclog::config::init_workspace(dir.path()).await.unwrap();
    fs::write(
        dir.path().join(".aclog/config.toml"),
        "[user]\nluogu_uid = \"123\"\nluogu_cookie = \"cookie\"\n\n[settings]\nmetadata_ttl_days = 7\nproblem_metadata_ttl_days = 7\nluogu_mappings_ttl_days = 7\nluogu_tags_ttl_days = 7\nreview_problem_interval_days = 21\npractice_tag_window_days = 60\npractice_tag_target_problems = 5\n",
    )
    .unwrap();
    dir
}

pub fn write_workspace_file(workspace: &Path, relative: &str, content: &str) -> PathBuf {
    let path = workspace.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path
}

pub fn sample_metadata(problem_id: &str) -> ProblemMetadata {
    sample_metadata_with_context(problem_id, "Luogu", None)
}

pub fn sample_metadata_with_context(
    problem_id: &str,
    source: &str,
    contest: Option<&str>,
) -> ProblemMetadata {
    ProblemMetadata {
        id: problem_id.to_string(),
        provider: aclog::problem::provider_from_problem_id(problem_id),
        title: format!("{problem_id} title"),
        difficulty: Some("入门".to_string()),
        tags: vec!["模拟".to_string(), "年份".to_string()],
        source: Some(source.to_string()),
        contest: contest.map(str::to_string),
        url: match aclog::problem::provider_from_problem_id(problem_id) {
            aclog::problem::ProblemProvider::AtCoder => {
                format!("https://atcoder.jp/contests/tasks/{problem_id}")
            }
            _ => format!("https://www.luogu.com.cn/problem/{problem_id}"),
        },
        fetched_at: FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .single()
            .unwrap(),
    }
}

pub fn sample_submission(submission_id: u64, verdict: &str) -> SubmissionRecord {
    sample_submission_for(
        submission_id,
        aclog::problem::ProblemProvider::Unknown,
        None,
        verdict,
    )
}

pub fn sample_submission_for(
    submission_id: u64,
    provider: aclog::problem::ProblemProvider,
    problem_id: Option<&str>,
    verdict: &str,
) -> SubmissionRecord {
    SubmissionRecord {
        submission_id,
        problem_id: problem_id.map(str::to_string),
        provider,
        submitter: "tester".to_string(),
        verdict: verdict.to_string(),
        score: Some(100),
        time_ms: Some(12),
        memory_mb: Some(1.5),
        submitted_at: Some(
            FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
                .single()
                .unwrap(),
        ),
    }
}

pub fn sample_atcoder_metadata(problem_id: &str, contest: Option<&str>) -> ProblemMetadata {
    sample_metadata_with_context(problem_id, "AtCoder", contest)
}

pub fn sample_atcoder_submission(
    submission_id: u64,
    problem_id: &str,
    verdict: &str,
) -> SubmissionRecord {
    sample_submission_for(
        submission_id,
        aclog::problem::ProblemProvider::AtCoder,
        Some(problem_id),
        verdict,
    )
}

pub fn sample_history_record(
    revision: &str,
    problem_id: &str,
    file_name: &str,
    submission_id: Option<u64>,
    verdict: &str,
) -> HistoricalSolveRecord {
    sample_history_record_with_context(
        revision,
        problem_id,
        file_name,
        submission_id,
        verdict,
        "Luogu",
        None,
    )
}

pub fn sample_history_record_with_context(
    revision: &str,
    problem_id: &str,
    file_name: &str,
    submission_id: Option<u64>,
    verdict: &str,
    source: &str,
    contest: Option<&str>,
) -> HistoricalSolveRecord {
    HistoricalSolveRecord {
        revision: revision.to_string(),
        record: SolveRecord {
            problem_id: problem_id.to_string(),
            provider: aclog::problem::provider_from_problem_id(problem_id),
            title: format!("{problem_id} title"),
            verdict: verdict.to_string(),
            score: Some(100),
            time_ms: Some(12),
            memory_mb: Some(1.5),
            difficulty: "入门".to_string(),
            tags: vec!["模拟".to_string()],
            source: source.to_string(),
            contest: contest.map(str::to_string),
            submission_id,
            submission_time: Some(
                FixedOffset::east_opt(8 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
                    .single()
                    .unwrap(),
            ),
            file_name: file_name.to_string(),
            training: TrainingFields::default(),
            source_order: 0,
        },
    }
}

pub fn active_change(path: &str) -> ProblemFileChange {
    ProblemFileChange {
        path: path.to_string(),
        kind: ProblemFileChangeKind::Active,
    }
}

pub fn deleted_change(path: &str) -> ProblemFileChange {
    ProblemFileChange {
        path: path.to_string(),
        kind: ProblemFileChangeKind::Deleted,
    }
}
