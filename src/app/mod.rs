pub mod deps;

mod browser;
mod init;
mod record_bind;
mod record_edit;
mod record_list;
mod record_rebind;
mod record_show;
mod stats;
pub(crate) mod support;
mod sync;

use std::path::PathBuf;

use color_eyre::Result;

use crate::ui::interaction::TerminalUi;

pub use self::browser::BrowserLaunchTarget;
use self::deps::LiveDeps;
pub use self::stats::StatsOptions;
pub use self::support::{RecordListQuery, TrainingFieldsPatch};
pub use self::sync::SyncOptions;
pub use crate::domain::browser::{BrowserQuery, BrowserRootView};

pub async fn run_init(workspace: PathBuf) -> Result<()> {
    init::run(workspace).await
}

pub async fn run_sync(workspace: PathBuf, options: SyncOptions) -> Result<()> {
    sync::run(workspace, options, &LiveDeps, &TerminalUi).await
}

pub async fn run_stats(workspace: PathBuf) -> Result<()> {
    stats::run(workspace, &StatsOptions::default(), &LiveDeps, &TerminalUi).await
}

pub async fn run_stats_with_options(workspace: PathBuf, options: StatsOptions) -> Result<()> {
    stats::run(workspace, &options, &LiveDeps, &TerminalUi).await
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

pub async fn run_record_list(workspace: PathBuf, query: support::RecordListQuery) -> Result<()> {
    record_list::run(workspace, &query, &LiveDeps).await
}

pub async fn run_record_browse(workspace: PathBuf, query: BrowserQuery) -> Result<()> {
    browser::run(workspace, query, &LiveDeps, &TerminalUi).await
}

pub async fn run_record_show(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    json: bool,
) -> Result<()> {
    record_show::run(workspace, file, record_rev, json, &LiveDeps).await
}

pub async fn run_record_edit(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    patch: support::TrainingFieldsPatch,
) -> Result<()> {
    record_edit::run(workspace, file, record_rev, patch, &LiveDeps).await
}

pub async fn run_sync_with<D>(
    workspace: PathBuf,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    sync::run(workspace, SyncOptions::default(), deps, ui).await
}

pub async fn run_sync_with_options<D>(
    workspace: PathBuf,
    dry_run: bool,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    sync::run(
        workspace,
        SyncOptions {
            dry_run,
            ..SyncOptions::default()
        },
        deps,
        ui,
    )
    .await
}

pub async fn run_sync_with_full_options<D>(
    workspace: PathBuf,
    options: SyncOptions,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    sync::run(workspace, options, deps, ui).await
}

pub async fn run_stats_with<D>(
    workspace: PathBuf,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    stats::run(workspace, &StatsOptions::default(), deps, ui).await
}

pub async fn run_stats_with_options_and_deps<D>(
    workspace: PathBuf,
    options: StatsOptions,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    stats::run(workspace, &options, deps, ui).await
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

pub async fn run_record_list_with<D>(
    workspace: PathBuf,
    query: support::RecordListQuery,
    deps: &D,
) -> Result<()>
where
    D: deps::AppDeps,
{
    record_list::run(workspace, &query, deps).await
}

pub async fn run_record_show_with<D>(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    json: bool,
    deps: &D,
) -> Result<()>
where
    D: deps::AppDeps,
{
    record_show::run(workspace, file, record_rev, json, deps).await
}

pub async fn run_record_browse_with<D>(
    workspace: PathBuf,
    query: BrowserQuery,
    deps: &D,
    ui: &impl crate::ui::interaction::UserInterface,
) -> Result<()>
where
    D: deps::AppDeps,
{
    browser::run(workspace, query, deps, ui).await
}

pub async fn run_record_edit_with<D>(
    workspace: PathBuf,
    file: PathBuf,
    record_rev: Option<String>,
    patch: support::TrainingFieldsPatch,
    deps: &D,
) -> Result<()>
where
    D: deps::AppDeps,
{
    record_edit::run(workspace, file, record_rev, patch, deps).await
}

pub fn render_record_list_output(records: &[crate::domain::record::FileRecordSummary]) -> String {
    record_list::render_output(records)
}
