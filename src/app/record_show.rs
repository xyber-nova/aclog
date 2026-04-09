use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::config::AclogPaths;

use super::{
    deps::AppDeps,
    support::{
        render_record_detail, render_record_detail_json, resolve_record_for_file,
        resolve_solution_file_target,
    },
};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display(), file = %file.display()))]
pub async fn run(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    json: bool,
    deps: &impl AppDeps,
) -> Result<()> {
    info!("开始查看记录详情");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_jj_workspace(&paths.workspace_root).await?;
    let target = resolve_solution_file_target(&paths, &file, deps).await?;
    let record = resolve_record_for_file(&paths, &target, record_rev.as_deref(), deps).await?;
    let output = if json {
        render_record_detail_json(&record)?
    } else {
        render_record_detail(&record)
    };
    deps.write_output(&output)?;

    info!(
        problem_id = target.problem_id,
        revision = record.revision,
        "记录详情已输出"
    );
    Ok(())
}
