use std::path::PathBuf;

use color_eyre::Result;
use tracing::{debug, info, instrument};

use crate::{config::AclogPaths, ui::interaction::UserInterface};

use super::deps::AppDeps;
use super::support::{
    history_records_for_file, rebind_selection_plan, resolve_solution_file_target,
    select_record_for_rebind, select_submission_for_record,
};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display(), file = %file.display()))]
pub async fn run(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    submission_id: Option<u64>,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始重绑记录");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_workspace().await?;
    let target = resolve_solution_file_target(&paths, &file, deps).await?;
    let selection_plan = rebind_selection_plan(record_rev.as_deref(), submission_id);
    let index = super::support::load_record_index(deps).await?;
    let candidates = history_records_for_file(
        index.all_records(),
        &target.repo_relative_path,
        &target.problem_id,
    );
    let selected_record =
        select_record_for_rebind(&target, &candidates, record_rev.as_deref(), deps, ui).await?;

    let config = crate::config::load_config(&paths)?;
    let metadata = deps
        .resolve_problem_metadata(&config, &paths, &target.problem_id)
        .await?;
    let submissions = deps
        .fetch_problem_submissions(&config, &paths, &target.problem_id)
        .await?;
    let record = select_submission_for_record(
        &target.problem_id,
        metadata.as_ref(),
        &submissions,
        submission_id,
        ui,
    )?;
    debug!(?selection_plan, "record rebind 交互计划已确定");
    let message = crate::commit_format::build_solve_commit_message_with_training(
        &target.problem_id,
        &target.repo_relative_path,
        metadata.as_ref(),
        &record,
        &selected_record.record.training,
    );
    deps.rewrite_commit_description(&selected_record.revision, &message)
        .await?;

    info!(
        problem_id = target.problem_id,
        revision = selected_record.revision,
        "重绑完成"
    );
    Ok(())
}
