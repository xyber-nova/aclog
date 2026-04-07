use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::ui::interaction::UserInterface;

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf, ui: &impl UserInterface) -> Result<()> {
    info!("开始统计");

    let paths = crate::config::AclogPaths::new(workspace)?;
    crate::vcs::ensure_jj_workspace(&paths.workspace_root)?;
    let config = crate::config::load_config(&paths)?;

    let messages = crate::vcs::collect_solve_commit_messages(&paths.workspace_root).await?;
    let records = crate::commit_format::parse_solve_records(&messages);
    let algorithm_tag_names = crate::api::load_algorithm_tag_names(&config, &paths).await?;
    let summary =
        crate::domain::stats::summarize_solve_records(&records, Some(&algorithm_tag_names));

    ui.show_stats(&paths.workspace_root, &summary)?;

    info!(
        total_solve_records = summary.total_solve_records,
        unique_problem_count = summary.unique_problem_count,
        "统计完成"
    );
    Ok(())
}
