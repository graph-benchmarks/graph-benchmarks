use anyhow::{bail, Result};
use common::config::parse_config;

use crate::args::Cli;

pub async fn destroy(cli: &Cli) -> Result<()> {
    let config = parse_config(&cli.file)?;
    for p in base_provider::PROVIDERS {
        if p.name() == config.setup.provider {
            p.destroy(&config.setup, cli.verbose).await?;
            return Ok(());
        }
    }
    bail!(format!("Unknown platform {}", config.setup.platform));
}
