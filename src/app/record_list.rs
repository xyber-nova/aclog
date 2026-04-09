use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::config::AclogPaths;

use super::{
    deps::AppDeps,
    support::{
        RecordListQuery, filter_record_summaries, load_record_index, render_record_list,
        render_record_list_json,
    },
};

pub fn render_output(records: &[crate::domain::record::FileRecordSummary]) -> String {
    render_record_list(records)
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf, query: &RecordListQuery, deps: &impl AppDeps) -> Result<()> {
    info!("开始列出已记录文件");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_jj_workspace(&paths.workspace_root).await?;
    let index = load_record_index(&paths, deps).await?;
    let summaries = index.current_by_file().to_vec();
    let mut tracked_summaries = Vec::with_capacity(summaries.len());
    for summary in summaries {
        if deps
            .is_tracked_file(&paths.workspace_root, &summary.file_name)
            .await?
        {
            tracked_summaries.push(summary);
        }
    }
    let filtered = filter_record_summaries(&tracked_summaries, query);
    let output = if query.json {
        render_record_list_json(&filtered)?
    } else {
        render_output(&filtered)
    };
    deps.write_output(&output)?;

    info!(records = filtered.len(), "已输出记录列表");
    Ok(())
}
