#![allow(dead_code)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use aclog::{
    app::deps::{OutputSink, ProblemProvider, RepoGateway},
    config::{AclogPaths, AppConfig},
    domain::{
        problem::ProblemMetadata,
        record::{HistoricalSolveRecord, SolveRecord, SyncSelection},
        stats::StatsSummary,
        submission::SubmissionRecord,
    },
    ui::interaction::UserInterface,
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
    algorithm_tag_names: Mutex<HashSet<String>>,
    solve_messages: Mutex<Vec<String>>,
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

    pub fn set_solve_messages(&self, messages: Vec<String>) {
        *self.solve_messages.lock().unwrap() = messages;
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
    ) -> Result<HashSet<String>> {
        Ok(self.algorithm_tag_names.lock().unwrap().clone())
    }
}

impl RepoGateway for FakeDeps {
    async fn ensure_jj_workspace(&self, _workspace_root: &Path) -> Result<()> {
        Ok(())
    }

    async fn collect_changed_problem_files(
        &self,
        _workspace_root: &Path,
    ) -> Result<Vec<ProblemFileChange>> {
        Ok(self.changed_files.lock().unwrap().clone())
    }

    async fn create_commits_for_files(
        &self,
        _workspace_root: &Path,
        commits: &[(String, String)],
    ) -> Result<()> {
        self.created_commits
            .lock()
            .unwrap()
            .extend(commits.iter().cloned());
        Ok(())
    }

    async fn collect_solve_commit_messages(&self, _workspace_root: &Path) -> Result<Vec<String>> {
        Ok(self.solve_messages.lock().unwrap().clone())
    }

    async fn collect_commit_descriptions(
        &self,
        _workspace_root: &Path,
    ) -> Result<Vec<(String, String)>> {
        Ok(self.commit_descriptions.lock().unwrap().clone())
    }

    async fn resolve_single_commit_id(
        &self,
        _workspace_root: &Path,
        revset_str: &str,
    ) -> Result<String> {
        self.resolved_revsets
            .lock()
            .unwrap()
            .get(revset_str)
            .cloned()
            .ok_or_else(|| eyre!("missing fake revision for `{revset_str}`"))
    }

    async fn is_tracked_file(
        &self,
        _workspace_root: &Path,
        repo_relative_path: &str,
    ) -> Result<bool> {
        Ok(self
            .tracked_files
            .lock()
            .unwrap()
            .contains(repo_relative_path))
    }

    async fn rewrite_commit_description(
        &self,
        _workspace_root: &Path,
        revision: &str,
        message: &str,
    ) -> Result<()> {
        self.rewritten_descriptions
            .lock()
            .unwrap()
            .push((revision.to_string(), message.to_string()));
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
    pub submission_selection: Mutex<Option<SyncSelection>>,
    pub record_submission_selection: Mutex<Option<Option<SubmissionRecord>>>,
    pub record_to_rebind_selection: Mutex<Option<Option<HistoricalSolveRecord>>>,
    pub delete_confirmation: Mutex<Option<SyncSelection>>,
    pub shown_stats: Mutex<Vec<StatsSummary>>,
}

impl FakeUi {
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
    fn select_submission(
        &self,
        _problem_id: &str,
        _metadata: Option<&ProblemMetadata>,
        _submissions: &[SubmissionRecord],
    ) -> Result<SyncSelection> {
        self.submission_selection
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| eyre!("unexpected select_submission call"))
    }

    fn select_record_submission(
        &self,
        _problem_id: &str,
        _metadata: Option<&ProblemMetadata>,
        _submissions: &[SubmissionRecord],
    ) -> Result<Option<SubmissionRecord>> {
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
        "[user]\nluogu_uid = \"123\"\nluogu_cookie = \"cookie\"\n\n[settings]\nmetadata_ttl_days = 7\nproblem_metadata_ttl_days = 7\nluogu_mappings_ttl_days = 7\nluogu_tags_ttl_days = 7\n",
    )
    .unwrap();
    dir
}

pub async fn init_real_workspace() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    aclog::config::init_workspace(dir.path()).await.unwrap();
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
    ProblemMetadata {
        id: problem_id.to_string(),
        title: format!("{problem_id} title"),
        difficulty: Some("入门".to_string()),
        tags: vec!["模拟".to_string(), "年份".to_string()],
        source: Some("Luogu".to_string()),
        url: format!("https://www.luogu.com.cn/problem/{problem_id}"),
        fetched_at: FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .single()
            .unwrap(),
    }
}

pub fn sample_submission(submission_id: u64, verdict: &str) -> SubmissionRecord {
    SubmissionRecord {
        submission_id,
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

pub fn sample_history_record(
    revision: &str,
    problem_id: &str,
    file_name: &str,
    submission_id: Option<u64>,
    verdict: &str,
) -> HistoricalSolveRecord {
    HistoricalSolveRecord {
        revision: revision.to_string(),
        record: SolveRecord {
            problem_id: problem_id.to_string(),
            title: format!("{problem_id} title"),
            verdict: verdict.to_string(),
            difficulty: "入门".to_string(),
            tags: vec!["模拟".to_string()],
            submission_id,
            submission_time: Some(
                FixedOffset::east_opt(8 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
                    .single()
                    .unwrap(),
            ),
            file_name: file_name.to_string(),
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
