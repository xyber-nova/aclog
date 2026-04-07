use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::config::AclogPaths;

use super::support::print_record_list;

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf) -> Result<()> {
    info!("开始列出已记录文件");

    let paths = AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let history_entries = crate::vcs::collect_commit_descriptions(&paths.workspace_root).await?;
    let history_records = crate::commit_format::parse_historical_solve_records(&history_entries);
    let summaries = crate::domain::stats::latest_records_by_file(&history_records);
    let mut tracked_summaries = Vec::with_capacity(summaries.len());
    for summary in summaries {
        if crate::vcs::is_tracked_file(&paths.workspace_root, &summary.file_name).await? {
            tracked_summaries.push(summary);
        }
    }
    print_record_list(&tracked_summaries);

    info!(records = tracked_summaries.len(), "已输出记录列表");
    Ok(())
}
