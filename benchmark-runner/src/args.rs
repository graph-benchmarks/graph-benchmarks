use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "git")]
#[command(author, version, about = "A graph benchmarking platform based on graphalytics", long_about = None)]
pub struct Cli {
    /// Verbose logging
    #[arg(long, short, action = ArgAction::SetTrue)]
    pub verbose: bool,

    /// Configuration file
    #[arg(long, short, default_value = "config.toml")]
    pub file: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Setup platform & kubernetes
    Setup(SetupArgs),
    /// Run benchmarks
    Benchmark(BenchmarkArgs),
    /// Teardown platform & kubernetes
    Destroy,
    /// List Resources
    Ls,
    /// Port forward dashboard
    Dashboard,
    /// Port forward postgres
    Postgres,
}

#[derive(Debug, Args)]
pub struct SetupArgs {
    /// Do not run create the platform resources, just set up their software
    #[arg(long, short, action = ArgAction::SetTrue)]
    pub only_software_setup: bool,
}

#[derive(Debug, Args)]
pub struct BenchmarkArgs {}
