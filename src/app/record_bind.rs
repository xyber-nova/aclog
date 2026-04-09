use std::path::PathBuf;

use color_eyre::Result;
use tracing::{debug, info, instrument};

use crate::{config::AclogPaths, ui::interaction::UserInterface};

use super::deps::AppDeps;
use super::support::{
    resolve_solution_file_target, select_submission_for_record, submission_selection_plan,
};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display(), file = %file.display()))]
pub async fn run(
    workspace: PathBuf,
    file: PathBuf,
    submission_id: Option<u64>,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始手工绑定记录");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_workspace().await?;
    let target = resolve_solution_file_target(&paths, &file, deps).await?;
    let needs_submission_choice = submission_selection_plan(submission_id);
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
    debug!(needs_submission_choice, "record bind 交互计划已确定");
    let message = crate::commit_format::build_solve_commit_message(
        &target.problem_id,
        &target.repo_relative_path,
        metadata.as_ref(),
        &record,
    );
    deps.create_commits(&[(target.repo_relative_path.clone(), message)])
        .await?;

    info!(problem_id = target.problem_id, "手工绑定完成");
    Ok(())
}
