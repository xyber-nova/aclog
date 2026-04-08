pub mod deps;

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

use self::deps::LiveDeps;

pub async fn run_init(workspace: PathBuf) -> Result<()> {
    init::run(workspace).await
}

pub async fn run_sync(workspace: PathBuf) -> Result<()> {
    sync::run(workspace, &LiveDeps, &TerminalUi).await
}

pub async fn run_stats(workspace: PathBuf) -> Result<()> {
    stats::run(workspace, &LiveDeps, &TerminalUi).await
}

pub async fn run_record_bind(
    workspace: PathBuf,
    file: PathBuf,
    submission_id: Option<u64>,
) -> Result<()> {
    record_bind::run(workspace, file, submission_id, &LiveDeps, &TerminalUi).await
}

pub async fn run_record_rebind(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    submission_id: Option<u64>,
) -> Result<()> {
    record_rebind::run(
        workspace,
        file,
        record_rev,
        submission_id,
        &LiveDeps,
        &TerminalUi,
    )
    .await
}

pub async fn run_record_list(workspace: PathBuf) -> Result<()> {
    record_list::run(workspace, &LiveDeps).await
}

pub async fn run_sync_with<D>(
    workspace: PathBuf,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    sync::run(workspace, deps, ui).await
}

pub async fn run_stats_with<D>(
    workspace: PathBuf,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    stats::run(workspace, deps, ui).await
}

pub async fn run_record_bind_with<D>(
    workspace: PathBuf,
    file: PathBuf,
    submission_id: Option<u64>,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    record_bind::run(workspace, file, submission_id, deps, ui).await
}

pub async fn run_record_rebind_with<D>(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    submission_id: Option<u64>,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    record_rebind::run(workspace, file, record_rev, submission_id, deps, ui).await
}

pub async fn run_record_list_with<D>(workspace: PathBuf, deps: &D) -> Result<()>
where
    D: deps::AppDeps,
{
    record_list::run(workspace, deps).await
}

pub fn render_record_list_output(records: &[crate::domain::record::FileRecordSummary]) -> String {
    record_list::render_output(records)
}
