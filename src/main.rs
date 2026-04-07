mod api;
mod app;
mod cli;
mod commit_format;
mod config;
mod domain;
mod models;
mod problem;
mod telemetry;
mod tui;
mod ui;
mod vcs;

use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_tracing()?;
    color_eyre::install()?;
    cli::run().await
}
