use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::config::AclogPaths;

use super::{
    deps::AppDeps,
    support::{
        TrainingFieldsPatch, apply_training_patch, resolve_record_for_file,
        resolve_solution_file_target,
    },
};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display(), file = %file.display()))]
pub async fn run(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    patch: TrainingFieldsPatch,
    deps: &impl AppDeps,
) -> Result<()> {
    info!("开始编辑训练字段");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_workspace().await?;
    let target = resolve_solution_file_target(&paths, &file, deps).await?;
    let record = resolve_record_for_file(&target, record_rev.as_deref(), deps).await?;
    let updated = apply_training_patch(&record, &patch)?;
    let message = crate::commit_format::build_solve_record_message(&updated);
    deps.rewrite_commit_description(&record.revision, &message)
        .await?;

    info!(
        problem_id = target.problem_id,
        revision = record.revision,
        "训练字段编辑完成"
    );
    Ok(())
}
