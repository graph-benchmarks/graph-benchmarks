use std::env;

use anyhow::Result;
use args::*;
use clap::Parser;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod args;
mod commands;
mod common;
mod config;
mod driver_config;
mod platforms;

#[macro_export]
macro_rules! exit {
    ($err:expr, $($arg:tt)*) => {
        {
            tracing::error!($($arg)*);
            anyhow::bail!($err)
        }
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(_) = env::var("LOG") {
        env::set_var("LOG", "error");
    }

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("LOG"))
        .init();

    let args = Cli::parse();
    match &args.command {
        Commands::Setup(setup) => commands::setup::setup(setup, &args).await,
        Commands::Benchmark(_) => commands::benchmark::run_benchmark(&args).await,
        Commands::Destroy => commands::destroy::destroy(&args).await,
    }?;

    Ok(())
}
