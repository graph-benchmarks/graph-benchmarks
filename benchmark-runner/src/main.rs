use std::env;

use anyhow::Result;
use args::*;
use clap::Parser;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod args;
mod commands;
mod rpc;
mod metrics_utils;
mod model;
mod schema;

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
        Commands::Ls => commands::ls::list(&args).await,
        Commands::Dashboard => commands::dashboard::access(&args).await,
    }?;

    Ok(())
}
