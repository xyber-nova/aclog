use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(workspace: PathBuf) -> Result<()> {
    info!("开始初始化");
    crate::config::init_workspace(&workspace).await?;
    info!("初始化完成");
    Ok(())
}
