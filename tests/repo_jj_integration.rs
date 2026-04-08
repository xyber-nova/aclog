mod support;

use std::{collections::HashSet, path::Path, sync::Mutex};

use aclog::{
    app::{
        deps::{LiveDeps, OutputSink, ProblemProvider, RepoGateway},
        run_record_list_with,
    },
    config::{AclogPaths, AppConfig},
    domain::{problem::ProblemMetadata, submission::SubmissionRecord},
    vcs,
};
use color_eyre::{Result, eyre::eyre};

use support::{init_real_workspace, write_workspace_file};

struct RealRepoOutputDeps {
    live: LiveDeps,
    outputs: Mutex<Vec<String>>,
}

impl Default for RealRepoOutputDeps {
    fn default() -> Self {
        Self {
            live: LiveDeps,
            outputs: Mutex::new(Vec::new()),
        }
    }
}

impl ProblemProvider for RealRepoOutputDeps {
    async fn resolve_problem_metadata(
        &self,
        _config: &AppConfig,
        _paths: &AclogPaths,
        _problem_id: &str,
    ) -> Result<Option<ProblemMetadata>> {
        Err(eyre!("problem provider is not used in this test"))
    }

    async fn fetch_problem_submissions(
        &self,
        _config: &AppConfig,
        _paths: &AclogPaths,
        _problem_id: &str,
    ) -> Result<Vec<SubmissionRecord>> {
        Err(eyre!("problem provider is not used in this test"))
    }

    async fn load_algorithm_tag_names(
        &self,
        _config: &AppConfig,
        _paths: &AclogPaths,
    ) -> Result<HashSet<String>> {
        Err(eyre!("problem provider is not used in this test"))
    }
}

impl RepoGateway for RealRepoOutputDeps {
    async fn ensure_jj_workspace(&self, workspace_root: &Path) -> Result<()> {
        self.live.ensure_jj_workspace(workspace_root).await
    }

    async fn collect_changed_problem_files(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<aclog::vcs::ProblemFileChange>> {
        self.live
            .collect_changed_problem_files(workspace_root)
            .await
    }

    async fn create_commits_for_files(
        &self,
        workspace_root: &Path,
        commits: &[(String, String)],
    ) -> Result<()> {
        self.live
            .create_commits_for_files(workspace_root, commits)
            .await
    }

    async fn collect_solve_commit_messages(&self, workspace_root: &Path) -> Result<Vec<String>> {
        self.live
            .collect_solve_commit_messages(workspace_root)
            .await
    }

    async fn collect_commit_descriptions(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<(String, String)>> {
        self.live.collect_commit_descriptions(workspace_root).await
    }

    async fn resolve_single_commit_id(
        &self,
        workspace_root: &Path,
        revset_str: &str,
    ) -> Result<String> {
        self.live
            .resolve_single_commit_id(workspace_root, revset_str)
            .await
    }

    async fn is_tracked_file(
        &self,
        workspace_root: &Path,
        repo_relative_path: &str,
    ) -> Result<bool> {
        self.live
            .is_tracked_file(workspace_root, repo_relative_path)
            .await
    }

    async fn rewrite_commit_description(
        &self,
        workspace_root: &Path,
        revision: &str,
        message: &str,
    ) -> Result<()> {
        self.live
            .rewrite_commit_description(workspace_root, revision, message)
            .await
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

    let changed = vcs::collect_changed_problem_files(workspace.path())
        .await
        .unwrap();

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].path, "P1001.cpp");
    assert_eq!(changed[0].kind, aclog::vcs::ProblemFileChangeKind::Active);
}

#[tokio::test]
async fn real_jj_commit_creation_tracking_and_rewrite_are_observable() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "P1002.cpp", "int main() {}\n");

    vcs::create_commits_for_files(
        workspace.path(),
        &[(
            "P1002.cpp".to_string(),
            "solve(P1002): Original\n\nSubmission-ID: 1\nFile: P1002.cpp".to_string(),
        )],
    )
    .await
    .unwrap();

    assert!(
        vcs::is_tracked_file(workspace.path(), "P1002.cpp")
            .await
            .unwrap()
    );

    let entries = vcs::collect_commit_descriptions(workspace.path())
        .await
        .unwrap();
    let (revision, _) = entries
        .into_iter()
        .find(|(_, description)| description.contains("solve(P1002): Original"))
        .unwrap();

    vcs::rewrite_commit_description(
        workspace.path(),
        &revision,
        "solve(P1002): Rewritten\n\nSubmission-ID: 2\nFile: P1002.cpp",
    )
    .await
    .unwrap();

    let rewritten_entries = vcs::collect_commit_descriptions(workspace.path())
        .await
        .unwrap();
    assert!(
        rewritten_entries
            .iter()
            .any(|(_, description)| description.contains("solve(P1002): Rewritten"))
    );
}

#[tokio::test]
async fn record_list_against_real_jj_history_matches_repository_truth() {
    let workspace = init_real_workspace().await;
    write_workspace_file(workspace.path(), "nested/P1003.cpp", "int main() {}\n");

    vcs::create_commits_for_files(
        workspace.path(),
        &[(
            "nested/P1003.cpp".to_string(),
            "solve(P1003): Real History\n\nVerdict: AC\nDifficulty: 入门\nSubmission-ID: 3\nSubmission-Time: 2024-01-02T03:04:05+08:00\nFile: nested/P1003.cpp".to_string(),
        )],
    )
    .await
    .unwrap();

    let deps = RealRepoOutputDeps::default();
    run_record_list_with(workspace.path().to_path_buf(), &deps)
        .await
        .unwrap();

    let output = deps.outputs.lock().unwrap().join("");
    assert!(output.contains("nested/P1003.cpp"));
    assert!(output.contains("P1003"));
    assert!(output.contains("AC"));
}
