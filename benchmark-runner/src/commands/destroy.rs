use anyhow::{bail, Result};

use crate::{args::Cli, config::parse_config, platforms::PLATFORMS};

pub async fn destroy(cli: &Cli) -> Result<()> {
    let config = parse_config(&cli.file)?;
    for p in PLATFORMS {
        if p.name() == config.setup.platform {
            p.destroy(cli.verbose).await?;
            return Ok(());
        }
    }
    bail!(format!("Unknown platform {}", config.setup.platform));
}
