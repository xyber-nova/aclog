mod init;
mod record_bind;
mod record_list;
mod record_rebind;
mod stats;
mod support;
mod sync;

use std::path::PathBuf;

use color_eyre::Result;

use crate::ui::interaction::TerminalUi;

pub async fn run_init(workspace: PathBuf) -> Result<()> {
    init::run(workspace).await
}

pub async fn run_sync(workspace: PathBuf) -> Result<()> {
    sync::run(workspace, &TerminalUi).await
}

pub async fn run_stats(workspace: PathBuf) -> Result<()> {
    stats::run(workspace, &TerminalUi).await
}

pub async fn run_record_bind(
    workspace: PathBuf,
    file: PathBuf,
    submission_id: Option<u64>,
) -> Result<()> {
    record_bind::run(workspace, file, submission_id, &TerminalUi).await
}

pub async fn run_record_rebind(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    submission_id: Option<u64>,
) -> Result<()> {
    record_rebind::run(workspace, file, record_rev, submission_id, &TerminalUi).await
}

pub async fn run_record_list(workspace: PathBuf) -> Result<()> {
    record_list::run(workspace).await
}
