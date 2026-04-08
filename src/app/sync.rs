use std::path::PathBuf;

use color_eyre::Result;
use tracing::{debug, info, instrument};

use crate::{config::AclogPaths, ui::interaction::UserInterface};

use super::deps::AppDeps;
use super::support::{planned_commit, selection_kind, should_fetch_submissions};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf, deps: &impl AppDeps, ui: &impl UserInterface) -> Result<()> {
    info!("开始同步");

    let paths = AclogPaths::new(workspace)?;
    let config = crate::config::load_config(&paths)?;
    deps.ensure_jj_workspace(&paths.workspace_root).await?;

    let changed_files = deps
        .collect_changed_problem_files(&paths.workspace_root)
        .await?;
    info!(changed_files = changed_files.len(), "已收集变更文件");
    debug!(?changed_files, "变更文件详情");

    let mut planned_commits = Vec::new();
    for change in changed_files {
        let file = change.path;
        let Some(problem_id) = crate::problem::extract_problem_id(&file) else {
            debug!(file, kind = ?change.kind, "跳过无法识别题号的文件");
            continue;
        };
        info!(file, problem_id, kind = ?change.kind, "处理变更题目文件");

        let metadata = deps
            .resolve_problem_metadata(&config, &paths, &problem_id)
            .await?;
        let selection = if should_fetch_submissions(change.kind) {
            let submissions = deps
                .fetch_problem_submissions(&config, &paths, &problem_id)
                .await?;
            info!(
                file,
                problem_id,
                kind = ?change.kind,
                submissions = submissions.len(),
                has_metadata = metadata.is_some(),
                "已获取同步上下文"
            );
            ui.select_submission(&problem_id, metadata.as_ref(), &submissions)?
        } else {
            info!(
                file,
                problem_id,
                kind = ?change.kind,
                has_metadata = metadata.is_some(),
                "已获取删除确认上下文"
            );
            ui.confirm_deleted_file(&problem_id, metadata.as_ref())?
        };
        info!(
            file,
            problem_id,
            selection = selection_kind(&selection),
            "同步选择完成"
        );
        if let Some(commit) = planned_commit(&problem_id, &file, metadata.as_ref(), &selection) {
            planned_commits.push(commit);
        }
    }

    info!(planned_commits = planned_commits.len(), "已生成提交计划");
    if !planned_commits.is_empty() {
        deps.create_commits_for_files(&paths.workspace_root, &planned_commits)
            .await?;
    }

    info!("同步完成");
    Ok(())
}
