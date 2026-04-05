use color_eyre::Result;
use tracing_subscriber::{
    EnvFilter, Layer as _, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

pub fn init_tracing() -> Result<()> {
    let env_filter =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("aclog=info"))?;

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .compact()
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_filter(env_filter),
        )
        .try_init()?;

    Ok(())
}
