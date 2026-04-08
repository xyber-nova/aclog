#![allow(async_fn_in_trait)]

use std::path::Path;

use color_eyre::Result;

use crate::{
    config::{AclogPaths, AppConfig},
    domain::{problem::ProblemMetadata, submission::SubmissionRecord},
    vcs::ProblemFileChange,
};

pub trait ProblemProvider {
    async fn resolve_problem_metadata(
        &self,
        config: &AppConfig,
        paths: &AclogPaths,
        problem_id: &str,
    ) -> Result<Option<ProblemMetadata>>;

    async fn fetch_problem_submissions(
        &self,
        config: &AppConfig,
        paths: &AclogPaths,
        problem_id: &str,
    ) -> Result<Vec<SubmissionRecord>>;

    async fn load_algorithm_tag_names(
        &self,
        config: &AppConfig,
        paths: &AclogPaths,
    ) -> Result<std::collections::HashSet<String>>;
}

pub trait RepoGateway {
    async fn ensure_jj_workspace(&self, workspace_root: &Path) -> Result<()>;
    async fn collect_changed_problem_files(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<ProblemFileChange>>;
    async fn create_commits_for_files(
        &self,
        workspace_root: &Path,
        commits: &[(String, String)],
    ) -> Result<()>;
    async fn collect_solve_commit_messages(&self, workspace_root: &Path) -> Result<Vec<String>>;
    async fn collect_commit_descriptions(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<(String, String)>>;
    async fn resolve_single_commit_id(
        &self,
        workspace_root: &Path,
        revset_str: &str,
    ) -> Result<String>;
    async fn is_tracked_file(
        &self,
        workspace_root: &Path,
        repo_relative_path: &str,
    ) -> Result<bool>;
    async fn rewrite_commit_description(
        &self,
        workspace_root: &Path,
        revision: &str,
        message: &str,
    ) -> Result<()>;
}

pub trait OutputSink {
    fn write_output(&self, text: &str) -> Result<()>;
}

pub trait AppDeps: ProblemProvider + RepoGateway + OutputSink {}

impl<T> AppDeps for T where T: ProblemProvider + RepoGateway + OutputSink {}

#[derive(Debug, Default, Clone, Copy)]
pub struct LiveDeps;

impl ProblemProvider for LiveDeps {
    async fn resolve_problem_metadata(
        &self,
        config: &AppConfig,
        paths: &AclogPaths,
        problem_id: &str,
    ) -> Result<Option<ProblemMetadata>> {
        crate::api::resolve_problem_metadata(config, paths, problem_id).await
    }

    async fn fetch_problem_submissions(
        &self,
        config: &AppConfig,
        paths: &AclogPaths,
        problem_id: &str,
    ) -> Result<Vec<SubmissionRecord>> {
        crate::api::fetch_problem_submissions(config, paths, problem_id).await
    }

    async fn load_algorithm_tag_names(
        &self,
        config: &AppConfig,
        paths: &AclogPaths,
    ) -> Result<std::collections::HashSet<String>> {
        crate::api::load_algorithm_tag_names(config, paths).await
    }
}

impl RepoGateway for LiveDeps {
    async fn ensure_jj_workspace(&self, workspace_root: &Path) -> Result<()> {
        crate::vcs::ensure_jj_workspace(workspace_root)
    }

    async fn collect_changed_problem_files(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<ProblemFileChange>> {
        crate::vcs::collect_changed_problem_files(workspace_root).await
    }

    async fn create_commits_for_files(
        &self,
        workspace_root: &Path,
        commits: &[(String, String)],
    ) -> Result<()> {
        crate::vcs::create_commits_for_files(workspace_root, commits).await
    }

    async fn collect_solve_commit_messages(&self, workspace_root: &Path) -> Result<Vec<String>> {
        crate::vcs::collect_solve_commit_messages(workspace_root).await
    }

    async fn collect_commit_descriptions(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<(String, String)>> {
        crate::vcs::collect_commit_descriptions(workspace_root).await
    }

    async fn resolve_single_commit_id(
        &self,
        workspace_root: &Path,
        revset_str: &str,
    ) -> Result<String> {
        crate::vcs::resolve_single_commit_id(workspace_root, revset_str).await
    }

    async fn is_tracked_file(
        &self,
        workspace_root: &Path,
        repo_relative_path: &str,
    ) -> Result<bool> {
        crate::vcs::is_tracked_file(workspace_root, repo_relative_path).await
    }

    async fn rewrite_commit_description(
        &self,
        workspace_root: &Path,
        revision: &str,
        message: &str,
    ) -> Result<()> {
        crate::vcs::rewrite_commit_description(workspace_root, revision, message).await
    }
}

impl OutputSink for LiveDeps {
    fn write_output(&self, text: &str) -> Result<()> {
        print!("{text}");
        Ok(())
    }
}
