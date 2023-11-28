use anyhow::{bail, Result};
use common::config::parse_config;

use crate::args::Cli;

pub async fn list(cli: &Cli) -> Result<()> {
    let config = parse_config(&cli.file)?;
    if let Some(p) = base_provider::PROVIDERS
        .iter()
        .find(|x| x.name() == config.setup.provider)
    {
        let info = p.platform_info(&config.setup, cli.verbose).await?;
        println!("{info:#?}");
    } else {
        bail!(format!("Unknown platform {}", config.setup.platform));
    }
    Ok(())
}
