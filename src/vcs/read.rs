use std::{collections::BTreeMap, path::Path};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt, eyre},
};
use futures::StreamExt as _;
use jj_lib::{
    commit::Commit,
    config::{ConfigLayer, ConfigSource, StackedConfig},
    fileset::FilesetAliasesMap,
    gitignore::GitIgnoreFile,
    matchers::{EverythingMatcher, NothingMatcher},
    merged_tree::MergedTree,
    repo::{ReadonlyRepo, Repo as _, StoreFactories},
    repo_path::RepoPathBuf,
    revset::{
        self, RevsetAliasesMap, RevsetDiagnostics, RevsetExtensions, RevsetIteratorExt,
        RevsetParseContext, SymbolResolver,
    },
    settings::UserSettings,
    workspace::{Workspace, default_working_copy_factories},
};
use tracing::{debug, info, instrument};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemFileChange {
    pub path: String,
    pub kind: ProblemFileChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemFileChangeKind {
    Active,
    Deleted,
}

#[instrument(level = "debug", skip_all, fields(workspace = %workspace_root.display()))]
pub fn ensure_jj_workspace(workspace_root: &Path) -> Result<()> {
    if workspace_root.join(".jj").is_dir() {
        debug!("已验证 jj 工作区");
        Ok(())
    } else {
        Err(eyre!("{} 中未找到 jj 工作区", workspace_root.display()))
    }
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace_root.display()))]
pub async fn collect_changed_problem_files(
    workspace_root: &Path,
) -> Result<Vec<ProblemFileChange>> {
    let (mut workspace, repo) = load_workspace_and_repo(workspace_root).await?;
    let workspace_name = workspace.workspace_name().to_owned();
    let current_commit = get_wc_commit(&repo, workspace_name.as_ref()).await?;
    let parent_commit = current_commit
        .parents()
        .await?
        .into_iter()
        .next()
        .ok_or_eyre("工作副本提交缺少父提交")?;

    let snapshot_tree = snapshot_working_copy(&mut workspace, repo.op_id()).await?;
    let mut diff_stream = parent_commit
        .tree()
        .diff_stream(&snapshot_tree, &EverythingMatcher);
    let mut files = BTreeMap::new();
    while let Some(diff) = diff_stream.next().await {
        let path = diff.path;
        if path.is_root() {
            continue;
        }
        let file_name = path.as_internal_file_string().to_string();
        if is_candidate_problem_file(&file_name) {
            let value_diff = diff
                .values
                .wrap_err_with(|| format!("读取 {file_name} 的变更详情失败"))?;
            let Some(kind) = classify_change_kind(
                value_diff.before.is_present(),
                value_diff.after.is_present(),
            ) else {
                continue;
            };
            files.insert(file_name, kind);
        }
    }
    let files = files
        .into_iter()
        .map(|(path, kind)| ProblemFileChange { path, kind })
        .collect::<Vec<_>>();
    info!(changed_files = files.len(), "已从 jj diff 收集变更题目文件");
    debug!(?files, "变更题目文件详情");
    Ok(files)
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace_root.display()))]
pub async fn collect_solve_commit_messages(workspace_root: &Path) -> Result<Vec<String>> {
    ensure_jj_workspace(workspace_root)?;
    let messages = collect_commit_descriptions(workspace_root)
        .await?
        .into_iter()
        .map(|(_, description)| description)
        .collect::<Vec<_>>();

    info!(messages = messages.len(), "已收集本地 solve 历史候选提交");
    debug!(?messages, "本地统计候选提交详情");
    Ok(messages)
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace_root.display()))]
pub async fn collect_commit_descriptions(workspace_root: &Path) -> Result<Vec<(String, String)>> {
    ensure_jj_workspace(workspace_root)?;
    let (_workspace, repo) = load_workspace_and_repo(workspace_root).await?;
    let commits = read_commits_for_revset(repo.as_ref(), "all()")?;
    let entries = commits
        .into_iter()
        .map(|commit| (commit.id().to_string(), commit.description().to_string()))
        .collect::<Vec<_>>();
    info!(commits = entries.len(), "已从 jj-lib 收集本地提交描述");
    Ok(entries)
}

#[instrument(level = "debug", skip_all, fields(workspace = %workspace_root.display(), revset = revset_str))]
pub async fn resolve_single_commit_id(workspace_root: &Path, revset_str: &str) -> Result<String> {
    ensure_jj_workspace(workspace_root)?;
    let (_workspace, repo) = load_workspace_and_repo(workspace_root).await?;
    let commits = read_commits_for_revset(repo.as_ref(), revset_str)?;
    match commits.as_slice() {
        [commit] => Ok(commit.id().to_string()),
        [] => Err(eyre!("revset `{revset_str}` 没有匹配到任何提交")),
        _ => Err(eyre!(
            "revset `{revset_str}` 匹配到了多条提交，无法唯一确定"
        )),
    }
}

#[instrument(level = "debug", skip_all, fields(workspace = %workspace_root.display(), file = repo_relative_path))]
pub async fn is_tracked_file(workspace_root: &Path, repo_relative_path: &str) -> Result<bool> {
    ensure_jj_workspace(workspace_root)?;
    let (mut workspace, repo) = load_workspace_and_repo(workspace_root).await?;
    let snapshot_tree = snapshot_working_copy_tracked_only(&mut workspace, repo.op_id()).await?;
    let repo_path = RepoPathBuf::from_internal_string(repo_relative_path)
        .wrap_err_with(|| format!("无效的仓库内路径：{repo_relative_path}"))?;
    let value = snapshot_tree.path_value(repo_path.as_ref()).await?;
    Ok(value.into_resolved().ok().flatten().is_some())
}

fn is_candidate_problem_file(path: &str) -> bool {
    !path.starts_with(".aclog/")
}

fn classify_change_kind(
    before_present: bool,
    after_present: bool,
) -> Option<ProblemFileChangeKind> {
    match (before_present, after_present) {
        (true, false) => Some(ProblemFileChangeKind::Deleted),
        (_, true) => Some(ProblemFileChangeKind::Active),
        (false, false) => None,
    }
}

async fn load_workspace_and_repo(
    workspace_root: &Path,
) -> Result<(Workspace, std::sync::Arc<ReadonlyRepo>)> {
    debug!(workspace = %workspace_root.display(), "正在加载 jj 工作区和仓库");
    let settings = default_user_settings()?;
    let store_factories = StoreFactories::default();
    let workspace = Workspace::load(
        &settings,
        workspace_root,
        &store_factories,
        &default_working_copy_factories(),
    )
    .wrap_err("加载 jj 工作区失败")?;
    let repo = workspace
        .repo_loader()
        .load_at_head()
        .await
        .wrap_err("加载 jj 仓库头部状态失败")?;
    debug!("jj 工作区和仓库加载完成");
    Ok((workspace, repo))
}

fn read_commits_for_revset(repo: &dyn jj_lib::repo::Repo, revset_str: &str) -> Result<Vec<Commit>> {
    let aliases_map = RevsetAliasesMap::new();
    let fileset_aliases_map = FilesetAliasesMap::new();
    let extensions = RevsetExtensions::default();
    let context = RevsetParseContext {
        aliases_map: &aliases_map,
        local_variables: Default::default(),
        user_email: "",
        date_pattern_context: chrono::Utc::now().fixed_offset().into(),
        default_ignored_remote: None,
        fileset_aliases_map: &fileset_aliases_map,
        use_glob_by_default: true,
        extensions: &extensions,
        workspace: None,
    };
    let mut diagnostics = RevsetDiagnostics::new();
    let user_expr = revset::parse(&mut diagnostics, revset_str, &context)
        .wrap_err_with(|| format!("解析 revset `{revset_str}` 失败"))?;
    let symbol_resolver = SymbolResolver::new(repo, extensions.symbol_resolvers());
    let revset = user_expr
        .resolve_user_expression(repo, &symbol_resolver)
        .wrap_err_with(|| format!("解析 revset `{revset_str}` 失败"))?
        .evaluate(repo)
        .wrap_err_with(|| format!("计算 revset `{revset_str}` 失败"))?;

    let mut commits = Vec::new();
    for item in revset.iter().commits(repo.store()) {
        commits.push(item.wrap_err("读取 revset 中的提交失败")?);
    }
    Ok(commits)
}

fn default_user_settings() -> Result<UserSettings> {
    let mut config = StackedConfig::with_defaults();
    let mut layer = ConfigLayer::empty(ConfigSource::User);
    layer.set_value("user.name", "")?;
    layer.set_value("user.email", "")?;
    layer.set_value("operation.hostname", "localhost")?;
    layer.set_value("operation.username", "aclog")?;
    layer.set_value("signing.behavior", "drop")?;
    config.add_layer(layer);
    Ok(UserSettings::from_config(config)?)
}

async fn get_wc_commit(
    repo: &std::sync::Arc<ReadonlyRepo>,
    workspace_name: &jj_lib::ref_name::WorkspaceName,
) -> Result<Commit> {
    let commit_id = repo
        .view()
        .get_wc_commit_id(workspace_name)
        .ok_or_eyre("工作区没有工作副本提交")?;
    Ok(repo.store().get_commit_async(commit_id).await?)
}

async fn snapshot_working_copy(
    workspace: &mut Workspace,
    operation_id: &jj_lib::op_store::OperationId,
) -> Result<MergedTree> {
    debug!("正在创建工作副本快照");
    let mut locked_ws = workspace
        .start_working_copy_mutation()
        .wrap_err("为快照锁定工作副本失败")?;
    let (tree, _stats) = locked_ws
        .locked_wc()
        .snapshot(&jj_lib::working_copy::SnapshotOptions {
            base_ignores: GitIgnoreFile::empty(),
            progress: None,
            start_tracking_matcher: &EverythingMatcher,
            force_tracking_matcher: &EverythingMatcher,
            max_new_file_size: 32 * 1024 * 1024,
        })
        .await
        .wrap_err("创建工作副本快照失败")?;
    locked_ws
        .finish(operation_id.clone())
        .await
        .wrap_err("持久化快照状态失败")?;
    debug!("工作副本快照已持久化");
    Ok(tree)
}

async fn snapshot_working_copy_tracked_only(
    workspace: &mut Workspace,
    operation_id: &jj_lib::op_store::OperationId,
) -> Result<MergedTree> {
    debug!("正在创建仅含已跟踪文件的工作副本快照");
    let mut locked_ws = workspace
        .start_working_copy_mutation()
        .wrap_err("为快照锁定工作副本失败")?;
    let (tree, _stats) = locked_ws
        .locked_wc()
        .snapshot(&jj_lib::working_copy::SnapshotOptions {
            base_ignores: GitIgnoreFile::empty(),
            progress: None,
            start_tracking_matcher: &NothingMatcher,
            force_tracking_matcher: &NothingMatcher,
            max_new_file_size: 32 * 1024 * 1024,
        })
        .await
        .wrap_err("创建工作副本快照失败")?;
    locked_ws
        .finish(operation_id.clone())
        .await
        .wrap_err("持久化快照状态失败")?;
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::{ProblemFileChangeKind, classify_change_kind};

    #[test]
    fn classify_change_kind_marks_deleted_when_only_before_exists() {
        assert_eq!(
            classify_change_kind(true, false),
            Some(ProblemFileChangeKind::Deleted)
        );
    }

    #[test]
    fn classify_change_kind_marks_active_when_after_exists() {
        assert_eq!(
            classify_change_kind(true, true),
            Some(ProblemFileChangeKind::Active)
        );
        assert_eq!(
            classify_change_kind(false, true),
            Some(ProblemFileChangeKind::Active)
        );
    }

    #[test]
    fn classify_change_kind_ignores_absent_entries() {
        assert_eq!(classify_change_kind(false, false), None);
    }
}
