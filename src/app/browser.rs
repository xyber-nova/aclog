use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::{
    config::AclogPaths,
    domain::browser::{
        BrowserQuery, BrowserRootView, build_browser_state, filter_browser_files,
        filter_browser_problems,
    },
    ui::interaction::UserInterface,
};

use super::{deps::AppDeps, support::load_record_index};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserLaunchTarget {
    Files,
    Problems,
    FileTimeline(String),
    ProblemTimeline(String),
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(
    workspace: PathBuf,
    query: BrowserQuery,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始打开记录浏览工作台");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_jj_workspace(&paths.workspace_root).await?;
    let index = load_record_index(&paths, deps).await?;
    let state = build_browser_state(&index);

    if query.json {
        let output = match query.root_view {
            BrowserRootView::Files => {
                serde_json::to_string_pretty(&filter_browser_files(&state.files, &query))?
            }
            BrowserRootView::Problems => {
                serde_json::to_string_pretty(&filter_browser_problems(&state.problems, &query))?
            }
        };
        deps.write_output(&(output + "\n"))?;
        return Ok(());
    }

    ui.open_record_browser(&paths.workspace_root, &query, &index)
}
