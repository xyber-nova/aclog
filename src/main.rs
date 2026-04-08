#![recursion_limit = "256"]

use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    aclog::telemetry::init_tracing()?;
    color_eyre::install()?;
    aclog::cli::run().await
}
