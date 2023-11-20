use anyhow::Result;

use crate::{args::Cli, config::{parse_config, Setup}};

use super::platform::{run_terraform_command, RunCommand, run_vagrant_command};

pub fn destroy(cli: &Cli) -> Result<()> {
    let config = parse_config(&cli.file)?;
    if let Setup::Platform(platform_args) = config.setup {
        let mut cmd = RunCommand {
            args: &["destroy", "--auto-approve"],
            ongoing: "Tearing down platform resources",
            success: "Destroyed platform resources",
            failure: "Could ont destroy platform resources",
        };
        match platform_args.platform.as_str() {
            "vagrant" => {
                cmd.args = &["destroy", "-f"];
                run_vagrant_command(cmd, cli.verbose)?
            },
            _ => run_terraform_command(&platform_args, cmd, cli.verbose)?
        }
    } else {
        println!("Pre-configured platform, so nothing to destroy!");
    }
    Ok(())
}