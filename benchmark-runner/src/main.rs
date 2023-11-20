use std::env;

use anyhow::Result;
use args::*;
use clap::Parser;
use console::{style, StyledObject};
use indicatif::ProgressStyle;

mod args;
mod commands;
mod config;
mod terraform_output;

lazy_static::lazy_static! {
    static ref DOTS_STYLE: ProgressStyle = ProgressStyle::with_template("{spinner} {msg} {elapsed_precise}").unwrap().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    static ref GREEN_TICK: StyledObject<&'static str> = style("✔").green();
}

#[macro_export]
macro_rules! exit {
    ($err:expr, $($arg:tt)*) => {
        {
            log::error!($($arg)*);
            anyhow::bail!($err)
        }
    };
}

fn main() -> Result<()> {
    if let Err(_) = env::var("LOG") {
        env::set_var("LOG", "error");
    }
    pretty_env_logger::init_custom_env("LOG");

    let args = Cli::parse();
    match &args.command {
        Commands::Setup(setup) => commands::setup::setup(setup, &args)?,
        Commands::Benchmark(_) => todo!(),
        Commands::Destroy => commands::destroy::destroy(&args)?,
    }

    Ok(())
}
