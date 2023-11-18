use std::{
    collections::HashMap,
    env,
    fs::{self, remove_file, set_permissions, Permissions},
    net::IpAddr,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Result};
use clap::{ArgAction, Parser};
use config::*;
use console::{style, StyledObject};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{error, info};

mod config;
mod terraform_output;

lazy_static::lazy_static! {
    static ref DOTS_STYLE: ProgressStyle = ProgressStyle::with_template("{spinner} {msg} {elapsed_precise}").unwrap().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    static ref GREEN_TICK: StyledObject<&'static str> = style("✔").green();
}

#[derive(Debug, Parser)]
#[command(name = "git")]
#[command(author, version, about = "A graph benchmarking platform based on graphalytics", long_about = None)]
struct Cli {
    /// Benchmark configuration file
    #[arg(long, short, default_value = "config.toml")]
    file: String,

    /// Verbose logging
    #[arg(long, short, action = ArgAction::SetTrue)]
    verbose: bool,

    /// Do not run terraform apply, just get outputs
    #[arg(long, short, action = ArgAction::SetTrue)]
    only_platform_outputs: bool,
}

macro_rules! exit {
    ($err:expr, $($arg:tt)*) => {
        {
            error!($($arg)*);
            bail!($err)
        }
    };
}

fn main() -> Result<()> {
    if let Err(_) = env::var("LOG") {
        env::set_var("LOG", "error");
    }
    pretty_env_logger::init_custom_env("LOG");

    let args = Cli::parse();
    let config = match std::fs::read_to_string(args.file.clone()) {
        Ok(s) => s,
        Err(err) => exit!(err, "Could not read config file {}", args.file),
    };

    let config: Result<Config, toml::de::Error> = toml::from_str(config.as_str());
    let config = match config {
        Ok(c) => c,
        Err(err) => exit!(err, "Could not parse config file {}", args.file),
    };

    info!("config file parsed");

    let connect_args = match &config.setup {
        Setup::PreConfiguredPlatform(p) => p.clone(),
        Setup::Platform(platform_args) => match setup_platform(platform_args, &args) {
            Ok(p) => p,
            Err(err) => exit!(err, "Could not setup platform"),
        },
    };
    info!("{connect_args:#?}");

    if connect_args.ips.len() < 2 {
        exit!(
            "Check platform setup output",
            "Need at least two nodes for kubernetes, only got {}",
            connect_args.ips.len()
        );
    }

    setup_master_node(&connect_args, &args)?;
    setup_worker_node(&connect_args, &args)?;

    Ok(())
}

fn setup_master_node(connect_args: &PlatformConnectInfo, args: &Cli) -> Result<()> {
    let master_hosts_file = PathBuf::from("k3s/inventory/master-hosts.ini");
    if master_hosts_file.exists() {
        remove_file(master_hosts_file.as_path())?;
    }

    fs::write(
        master_hosts_file,
        format!(
            "[master]\n{}\n\n[all:vars]\nansible_user={}",
            connect_args.ips[0],
            connect_args
                .host_username
                .clone()
                .unwrap_or("root".to_owned())
        ),
    )?;

    let mut cmd = Command::new("ansible-playbook");
    let mut setup_master = cmd
        .current_dir("k3s")
        .arg("master.yaml")
        .arg("--private-key")
        .arg(&connect_args.private_key_file)
        .arg("-i")
        .arg("inventory/master-hosts.ini");
    if !args.verbose {
        setup_master = setup_master.stdout(Stdio::piped());
    }
    let setup_master = setup_master.spawn()?;

    let mut m = None;
    let mut master_p = None;
    if !args.verbose {
        m = Some(MultiProgress::new());
        master_p = Some(m.as_ref().unwrap().add(ProgressBar::new_spinner()));
        let w = master_p.as_ref().unwrap();
        w.set_style(DOTS_STYLE.clone());
        w.enable_steady_tick(Duration::from_millis(80));
        w.set_message("Setting up master node");
    }

    let output = setup_master.wait_with_output()?;
    if !output.status.success() {
        exit!(
            String::from_utf8(output.stdout)?,
            "Could not setup master node"
        );
    }

    if master_p.is_some() {
        master_p.as_ref().unwrap().finish_and_clear();
        m.as_ref().unwrap().clear()?;
    }
    println!("{} {}", GREEN_TICK.to_string(), "Master node ready");
    Ok(())
}

fn setup_worker_node(connect_args: &PlatformConnectInfo, args: &Cli) -> Result<()> {
    let mut cmd = Command::new("ansible-playbook");
    let mut setup_master = cmd
        .current_dir("k3s")
        .arg("worker.yaml")
        .arg("--private-key")
        .arg(&connect_args.private_key_file)
        .arg("-i")
        .arg("inventory/worker-hosts.ini");
    if !args.verbose {
        setup_master = setup_master.stdout(Stdio::piped());
    }
    let setup_master = setup_master.spawn()?;

    let mut m = None;
    let mut worker_p = None;
    if !args.verbose {
        m = Some(MultiProgress::new());
        worker_p = Some(m.as_ref().unwrap().add(ProgressBar::new_spinner()));
        let w = worker_p.as_ref().unwrap();
        w.set_style(DOTS_STYLE.clone());
        w.enable_steady_tick(Duration::from_millis(80));
        w.set_message("Setting up worker node");
    }

    let worker_hosts_file = PathBuf::from("k3s/inventory/worker-hosts.ini");
    if worker_hosts_file.exists() {
        remove_file(worker_hosts_file.as_path())?;
    }

    let mut hosts_str = String::from("[workers]\n");
    hosts_str.push_str(
        &connect_args
            .ips
            .iter()
            .skip(1)
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("\n"),
    );
    hosts_str.push_str(&format!(
        "\n\n[workers:vars]\nmaster={}\nnode_token={}\nansible_user={}",
        connect_args.ips[0],
        fs::read_to_string("k3s/node-token")?,
        connect_args
            .host_username
            .clone()
            .unwrap_or("root".to_owned())
    ));
    fs::write(worker_hosts_file.as_path(), hosts_str)?;

    let output = setup_master.wait_with_output()?;
    if !output.status.success() {
        exit!(
            String::from_utf8(output.stdout)?,
            "Could not setup worker node"
        );
    }

    if worker_p.is_some() {
        worker_p.as_ref().unwrap().finish_and_clear();
        m.as_ref().unwrap().clear()?;
    }
    println!("{} {}", GREEN_TICK.to_string(), "Worker nodes ready");
    Ok(())
}

fn setup_platform(args: &PlatformArgs, cli: &Cli) -> Result<PlatformConnectInfo> {
    if !cli.only_platform_outputs {
        let max_nodes = args.node_configs.iter().max().unwrap();
        let mut vm_map: HashMap<String, HashMap<String, String>> = HashMap::new();
        for i in 0..*max_nodes {
            let mut m: HashMap<String, String> = HashMap::new();
            if i == 0 {
                m.insert("name".to_owned(), "master".to_owned());
            } else {
                m.insert("name".to_owned(), format!("worker-{}", i + 1));
            }

            if args.platform_env.is_some() {
                args.platform_env
                    .as_ref()
                    .unwrap()
                    .iter()
                    .for_each(|(k, v)| {
                        m.insert(k.to_owned(), v.to_owned());
                    });
            }
            vm_map.insert(i.to_string(), m);
        }

        let mut cmd = Command::new("terraform");
        let mut terraform_setup = cmd
            .current_dir(format!("platform/{}", args.platform))
            .env("vm_map", serde_json::to_string(&vm_map)?)
            .arg("apply")
            .arg("-var")
            .arg("vm_map=$vm_map")
            .arg("--auto-approve");
        let mut pb = None;
        if !cli.verbose {
            terraform_setup = terraform_setup.stdout(Stdio::piped());
            pb = Some(ProgressBar::new_spinner());
            let w = pb.as_ref().unwrap();
            w.set_style(DOTS_STYLE.clone());
            w.enable_steady_tick(Duration::from_millis(80));
            w.set_message(format!(
                "Spinning up platform resources ({})",
                args.platform
            ));
        }

        let terraform_setup = terraform_setup.spawn()?;
        let output = terraform_setup.wait_with_output()?;
        if !output.status.success() {
            exit!(
                String::from_utf8(output.stdout)?,
                "Could not setup platform resources"
            );
        }

        let msg = format!("Platform resources ({}) up", args.platform);
        if !cli.verbose {
            if let Some(pb) = pb {
                pb.finish_and_clear();
            }
        }
        println!("{} {}", GREEN_TICK.to_string(), msg);
    }

    info!("Skipping platform setup");

    let mut cmd = Command::new("terraform");
    let output = cmd
        .current_dir(format!("platform/{}", args.platform))
        .arg("output")
        .arg("-json")
        .output()?;
    let json = String::from_utf8(output.stdout)?;
    let mut connect_info: terraform_output::Root = serde_json::from_str(&json)?;

    let private_key_file = format!("platform/{}/key.pem", args.platform);
    std::fs::write(&private_key_file, connect_info.key_data.value)?;
    set_permissions(&private_key_file, Permissions::from_mode(0o600))?;

    let ips: Result<Vec<IpAddr>, _> = connect_info
        .linux_virtual_machine_ips
        .value
        .drain(..)
        .map(|x| x.first().unwrap().parse::<IpAddr>())
        .collect();
    Ok(PlatformConnectInfo {
        private_key_file: format!("../{}", private_key_file),
        ips: ips?,
        host_username: args.host_username.clone(),
    })
}
