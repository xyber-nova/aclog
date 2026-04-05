mod api;
mod cli;
mod config;
mod models;
mod problem;
mod telemetry;
mod tui;
mod vcs;

use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_tracing()?;
    color_eyre::install()?;
    cli::run().await
}
