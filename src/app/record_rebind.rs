use std::path::PathBuf;

use color_eyre::Result;
use tracing::{debug, info, instrument};

use crate::{config::AclogPaths, ui::interaction::UserInterface};

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
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始重绑记录");

    let paths = AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let target = resolve_solution_file_target(&paths, &file).await?;
    let selection_plan = rebind_selection_plan(record_rev.as_deref(), submission_id);
    let history_entries = crate::vcs::collect_commit_descriptions(&paths.workspace_root).await?;
    let history_records = crate::commit_format::parse_historical_solve_records(&history_entries);
    let candidates = history_records_for_file(
        &history_records,
        &target.repo_relative_path,
        &target.problem_id,
    );
    let selected_record =
        select_record_for_rebind(&paths, &target, &candidates, record_rev.as_deref(), ui).await?;

    let config = crate::config::load_config(&paths)?;
    let metadata =
        crate::api::resolve_problem_metadata(&config, &paths, &target.problem_id).await?;
    let submissions =
        crate::api::fetch_problem_submissions(&config, &paths, &target.problem_id).await?;
    let record = select_submission_for_record(
        &target.problem_id,
        metadata.as_ref(),
        &submissions,
        submission_id,
        ui,
    )?;
    debug!(?selection_plan, "record rebind 交互计划已确定");
    let message = crate::commit_format::build_solve_commit_message(
        &target.problem_id,
        &target.repo_relative_path,
        metadata.as_ref(),
        &record,
    );
    crate::vcs::rewrite_commit_description(
        &paths.workspace_root,
        &selected_record.revision,
        &message,
    )
    .await?;

    info!(
        problem_id = target.problem_id,
        revision = selected_record.revision,
        "重绑完成"
    );
    Ok(())
}
