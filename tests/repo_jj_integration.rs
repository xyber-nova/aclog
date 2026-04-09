mod support;

use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use aclog::{
    app::{
        BrowserQuery, BrowserRootView, RecordListQuery, SyncOptions, TrainingFieldsPatch,
        deps::{JjRepository, LiveDeps, OutputSink, ProblemProvider},
        run_record_browse_with, run_record_edit_with, run_record_list_with,
        run_sync_with_full_options,
    },
    config::{AclogPaths, AppConfig},
    domain::{problem::ProblemMetadata, submission::SubmissionRecord},
    vcs::JjRepoActorHandle,
};
use color_eyre::Result;

use support::{
    FakeUi, init_real_workspace, sample_metadata, sample_submission, write_workspace_file,
};

struct RealRepoOutputDeps {
    live: LiveDeps,
    metadata_by_problem: Mutex<HashMap<String, Option<ProblemMetadata>>>,
    submissions_by_problem: Mutex<HashMap<String, Vec<SubmissionRecord>>>,
    algorithm_tag_names: Mutex<HashSet<String>>,
    outputs: Mutex<Vec<String>>,
}

impl RealRepoOutputDeps {
    fn new(workspace_root: &std::path::Path) -> Self {
        Self {
            live: LiveDeps::new(workspace_root.to_path_buf()),
            metadata_by_problem: Mutex::new(HashMap::new()),
            submissions_by_problem: Mutex::new(HashMap::new()),
            algorithm_tag_names: Mutex::new(HashSet::new()),
            outputs: Mutex::new(Vec::new()),
        }
    }

    fn insert_metadata(&self, problem_id: &str, metadata: Option<ProblemMetadata>) {
        self.metadata_by_problem
            .lock()
            .unwrap()
            .insert(problem_id.to_string(), metadata);
    }

    fn insert_submissions(&self, problem_id: &str, submissions: Vec<SubmissionRecord>) {
        self.submissions_by_problem
            .lock()
            .unwrap()
            .insert(problem_id.to_string(), submissions);
    }
}

impl ProblemProvider for RealRepoOutputDeps {
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

impl JjRepository for RealRepoOutputDeps {
    async fn ensure_workspace(&self) -> Result<()> {
        self.live.ensure_workspace().await
    }

    async fn detect_working_copy_changes(&self) -> Result<Vec<aclog::vcs::ProblemFileChange>> {
        self.live.detect_working_copy_changes().await
    }

    async fn load_record_index(&self) -> Result<aclog::domain::record_index::RecordIndex> {
        self.live.load_record_index().await
    }

    async fn resolve_revision(&self, revset_str: &str) -> Result<String> {
        self.live.resolve_revision(revset_str).await
    }

    async fn is_tracked_file(&self, repo_relative_path: &str) -> Result<bool> {
        self.live.is_tracked_file(repo_relative_path).await
    }

    async fn create_commits(&self, commits: &[(String, String)]) -> Result<()> {
        self.live.create_commits(commits).await
    }

    async fn rewrite_commit_description(&self, revision: &str, message: &str) -> Result<()> {
        self.live.rewrite_commit_description(revision, message).await
    }
}

impl OutputSink for RealRepoOutputDeps {
    fn write_output(&self, text: &str) -> Result<()> {
        self.outputs.lock().unwrap().push(text.to_string());
        Ok(())
    }
}

#[tokio::test]
async fn real_jj_workspace_initializes_and_detects_changed_problem_files() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "P1001.cpp", "int main() {}\n");
    let repo = JjRepoActorHandle::for_workspace(workspace.path().to_path_buf());

    let changed = repo.detect_working_copy_changes().await.unwrap();

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].path, "P1001.cpp");
    assert_eq!(changed[0].kind, aclog::vcs::ProblemFileChangeKind::Active);
}

#[tokio::test]
async fn real_jj_commit_creation_tracking_and_rewrite_are_observable() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "P1002.cpp", "int main() {}\n");
    let repo = JjRepoActorHandle::for_workspace(workspace.path().to_path_buf());

    repo.create_commits(&[(
            "P1002.cpp".to_string(),
            "solve(P1002): Original\n\nSubmission-ID: 1\nFile: P1002.cpp".to_string(),
        )])
        .await
        .unwrap();

    assert!(repo.is_tracked_file("P1002.cpp").await.unwrap());

    let index = repo.load_record_index().await.unwrap();
    let revision = index.current_by_file()[0].revision.clone();

    repo.rewrite_commit_description(
        &revision,
        "solve(P1002): Rewritten\n\nSubmission-ID: 2\nFile: P1002.cpp",
    )
    .await
    .unwrap();

    let rewritten_index = repo.load_record_index().await.unwrap();
    assert_eq!(rewritten_index.current_by_file()[0].title, "Rewritten");
    assert_eq!(rewritten_index.current_by_file()[0].submission_id, Some(2));
}

#[tokio::test]
async fn record_list_against_real_jj_history_matches_repository_truth() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "nested/P1003.cpp", "int main() {}\n");

    let repo = JjRepoActorHandle::for_workspace(workspace.path().to_path_buf());
    repo.create_commits(&[(
            "nested/P1003.cpp".to_string(),
            "solve(P1003): Real History\n\nVerdict: AC\nDifficulty: 入门\nSubmission-ID: 3\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: nested/P1003.cpp".to_string(),
        )])
        .await
        .unwrap();

    let deps = RealRepoOutputDeps::new(workspace.path());
    run_record_list_with(
        workspace.path().to_path_buf(),
        RecordListQuery::default(),
        &deps,
    )
    .await
    .unwrap();

    let output = deps.outputs.lock().unwrap().join("");
    assert!(output.contains("nested/P1003.cpp"));
    assert!(output.contains("P1003"));
    assert!(output.contains("AC"));
}

#[tokio::test]
async fn real_jj_record_edit_rewrites_training_notes() {
    let workspace = init_real_workspace().await;
    let file = write_workspace_file(workspace.path(), "P1004.cpp", "int main() {}\n");

    let repo = JjRepoActorHandle::for_workspace(workspace.path().to_path_buf());
    repo.create_commits(&[(
            "P1004.cpp".to_string(),
            "solve(P1004): Original\n\nVerdict: WA\nSubmission-ID: 1\nFile: P1004.cpp".to_string(),
        )])
        .await
        .unwrap();

    let deps = RealRepoOutputDeps::new(workspace.path());
    run_record_edit_with(
        workspace.path().to_path_buf(),
        file,
        None,
        TrainingFieldsPatch {
            note: Some("补上图论复盘".to_string()),
            ..TrainingFieldsPatch::default()
        },
        &deps,
    )
    .await
    .unwrap();

    let index = deps.live.load_record_index().await.unwrap();
    assert_eq!(
        index.current_by_file()[0].training.note.as_deref(),
        Some("补上图论复盘")
    );
}

#[tokio::test]
async fn real_jj_record_browse_json_reads_history_views() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "nested/P1005.cpp", "int main() {}\n");

    let repo = JjRepoActorHandle::for_workspace(workspace.path().to_path_buf());
    repo.create_commits(&[(
            "nested/P1005.cpp".to_string(),
            "solve(P1005): Real Browser\n\nVerdict: AC\nDifficulty: 入门\nTags: 模拟\nSubmission-ID: 5\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: nested/P1005.cpp".to_string(),
        )])
        .await
        .unwrap();

    let deps = RealRepoOutputDeps::new(workspace.path());
    let ui = FakeUi::default();
    run_record_browse_with(
        workspace.path().to_path_buf(),
        BrowserQuery {
            root_view: BrowserRootView::Problems,
            json: true,
            ..BrowserQuery::default()
        },
        &deps,
        &ui,
    )
    .await
    .unwrap();

    let output = deps.outputs.lock().unwrap().join("");
    assert!(output.contains("\"problem_id\": \"P1005\""));
    assert!(output.contains("\"files\""));
}

#[tokio::test]
async fn real_jj_sync_resume_restores_saved_batch() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "P1006.cpp", "int main() {}\n");

    let deps = RealRepoOutputDeps::new(workspace.path());
    deps.insert_metadata("P1006", Some(sample_metadata("P1006")));
    deps.insert_submissions("P1006", vec![sample_submission(88, "AC")]);

    let paused_ui = FakeUi {
        sync_batch_review_selection: Mutex::new(vec![None]),
        submission_selection: Mutex::new(Some(aclog::domain::record::SyncSelection::Submission(
            sample_submission(88, "AC"),
        ))),
        ..FakeUi::default()
    };
    run_sync_with_full_options(
        workspace.path().to_path_buf(),
        SyncOptions::default(),
        &deps,
        &paused_ui,
    )
    .await
    .unwrap();
    assert!(workspace.path().join(".aclog/sync-session.toml").exists());

    let resumed_ui = FakeUi {
        submission_selection: Mutex::new(Some(aclog::domain::record::SyncSelection::Submission(
            sample_submission(88, "AC"),
        ))),
        ..FakeUi::default()
    };
    run_sync_with_full_options(
        workspace.path().to_path_buf(),
        SyncOptions {
            resume: true,
            ..SyncOptions::default()
        },
        &deps,
        &resumed_ui,
    )
    .await
    .unwrap();

    assert!(!workspace.path().join(".aclog/sync-session.toml").exists());
    let index = deps.live.load_record_index().await.unwrap();
    assert_eq!(index.current_by_file()[0].problem_id, "P1006");
}
