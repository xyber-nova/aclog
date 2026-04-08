use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::config::AclogPaths;

use super::{deps::AppDeps, support::render_record_list};

pub fn render_output(records: &[crate::domain::record::FileRecordSummary]) -> String {
    render_record_list(records)
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf, deps: &impl AppDeps) -> Result<()> {
    info!("开始列出已记录文件");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_jj_workspace(&paths.workspace_root).await?;
    let history_entries = deps
        .collect_commit_descriptions(&paths.workspace_root)
        .await?;
    let history_records = crate::commit_format::parse_historical_solve_records(&history_entries);
    let summaries = crate::domain::stats::latest_records_by_file(&history_records);
    let mut tracked_summaries = Vec::with_capacity(summaries.len());
    for summary in summaries {
        if deps
            .is_tracked_file(&paths.workspace_root, &summary.file_name)
            .await?
        {
            tracked_summaries.push(summary);
        }
    }
    deps.write_output(&render_output(&tracked_summaries))?;

    info!(records = tracked_summaries.len(), "已输出记录列表");
    Ok(())
}
