#![allow(async_fn_in_trait)]

use std::path::PathBuf;

use color_eyre::Result;

use crate::{
    config::{AclogPaths, AppConfig},
    domain::{problem::ProblemMetadata, record_index::RecordIndex, submission::SubmissionRecord},
    problem::ProblemProvider as ProblemSource,
    vcs::{JjRepoActorHandle, ProblemFileChange},
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
        provider: ProblemSource,
    ) -> Result<std::collections::HashSet<String>>;
}

pub trait JjRepository {
    async fn ensure_workspace(&self) -> Result<()>;
    async fn detect_working_copy_changes(&self) -> Result<Vec<ProblemFileChange>>;
    async fn load_record_index(&self) -> Result<RecordIndex>;
    async fn resolve_revision(&self, revset_str: &str) -> Result<String>;
    async fn is_tracked_file(&self, repo_relative_path: &str) -> Result<bool>;
    async fn create_commits(&self, commits: &[(String, String)]) -> Result<()>;
    async fn rewrite_commit_description(&self, revision: &str, message: &str) -> Result<()>;
}

pub trait OutputSink {
    fn write_output(&self, text: &str) -> Result<()>;
}

pub trait AppDeps: ProblemProvider + JjRepository + OutputSink {}

impl<T> AppDeps for T where T: ProblemProvider + JjRepository + OutputSink {}

#[derive(Debug, Clone)]
pub struct LiveDeps {
    repo: JjRepoActorHandle,
}

impl LiveDeps {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            repo: JjRepoActorHandle::for_workspace(workspace_root),
        }
    }
}

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
        provider: ProblemSource,
    ) -> Result<std::collections::HashSet<String>> {
        crate::api::load_algorithm_tag_names(config, paths, provider).await
    }
}

impl JjRepository for LiveDeps {
    async fn ensure_workspace(&self) -> Result<()> {
        self.repo.ensure_workspace().await
    }

    async fn detect_working_copy_changes(&self) -> Result<Vec<ProblemFileChange>> {
        self.repo.detect_working_copy_changes().await
    }

    async fn load_record_index(&self) -> Result<RecordIndex> {
        self.repo.load_record_index().await
    }

    async fn resolve_revision(&self, revset_str: &str) -> Result<String> {
        self.repo.resolve_revision(revset_str).await
    }

    async fn is_tracked_file(&self, repo_relative_path: &str) -> Result<bool> {
        self.repo.is_tracked_file(repo_relative_path).await
    }

    async fn create_commits(&self, commits: &[(String, String)]) -> Result<()> {
        self.repo.create_commits(commits).await
    }

    async fn rewrite_commit_description(&self, revision: &str, message: &str) -> Result<()> {
        self.repo
            .rewrite_commit_description(revision, message)
            .await
    }
}

impl OutputSink for LiveDeps {
    fn write_output(&self, text: &str) -> Result<()> {
        print!("{text}");
        Ok(())
    }
}
