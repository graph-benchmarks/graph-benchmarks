use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, remove_file};
use tracing::info;

use crate::{
    args::*,
    common::command,
    config::{self, *},
    exit,
    platforms::PLATFORMS,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Item<'a> {
    pub hosts: HashMap<String, ()>,
    #[serde(borrow)]
    pub vars: HashMap<&'a str, String>,
}

async fn setup_master_node(
    connect_args: &PlatformConnectInfo,
    kube_config: Option<config::KubeSetup>,
    drivers: Vec<String>,
    verbose: bool,
) -> Result<()> {
    let master_hosts_file = PathBuf::from("k3s/inventory/master-hosts.yaml");
    if master_hosts_file.exists() {
        remove_file(master_hosts_file.as_path()).await?;
    }

    let hosts = HashMap::from([(connect_args.master_ip.to_string(), ())]);
    let mut vars = HashMap::from([(
        "ansible_user",
        connect_args
            .host_username
            .clone()
            .unwrap_or("root".to_owned()),
    )]);

    if kube_config.is_some() {
        let k = kube_config.as_ref().unwrap();
        if k.dashboard.is_some() && *k.dashboard.as_ref().unwrap() {
            vars.insert("dashboard", "1".to_owned());
        }
    }

    let master_hosts = HashMap::from([("master", Item { hosts, vars })]);
    fs::write(master_hosts_file, serde_yaml::to_string(&master_hosts)?).await?;

    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }
    command(
        "ansible-playbook",
        &[
            "main-master.yaml",
            "--private-key",
            &connect_args.private_key_file,
            "-i",
            "inventory/master-hosts.yaml",
        ],
        verbose,
        [
            "Setting up master node",
            "Could not setup master node",
            "Master node ready",
        ],
        "k3s",
        env.clone(),
    )
    .await?;

    let kube_config = fs::read_to_string("k3s/kube-config").await?;
    let kube_config = kube_config.replace(
        "https://127.0.0.1:6443",
        &format!("https://{}:6443", connect_args.master_ip),
    );
    fs::write("k3s/kube-config", kube_config).await?;

    // TODO: load metrics & visualization containers
    for driver in drivers {
        command(
            "ansible-playbook",
            &[
                "load-image.yaml",
                "--private-key",
                &connect_args.private_key_file,
                "-i",
                "inventory/master-hosts.yaml",
                "--extra-vars",
                &format!("image={driver}.tar")
            ],
            verbose,
            [
                &format!("Loading {driver} driver into kubernetes"),
                &format!("Could not load {driver} driver into kubernetes"),
                &format!("Done loading {driver} driver into kubernetes"),
            ],
            "k3s",
            env.clone(),
        )
        .await?;
    }
    Ok(())
}

async fn setup_worker_node(connect_args: &PlatformConnectInfo, verbose: bool) -> Result<()> {
    let mut hosts = HashMap::new();
    connect_args.worker_ips.iter().for_each(|x| {
        hosts.insert(x.to_string(), ());
    });
    let vars = HashMap::from([
        ("master", connect_args.master_ip.to_string()),
        (
            "node_token",
            fs::read_to_string("k3s/node-token")
                .await?
                .trim()
                .to_owned(),
        ),
        (
            "ansible_user",
            connect_args
                .host_username
                .clone()
                .unwrap_or("root".to_owned()),
        ),
    ]);
    let worker_hosts = HashMap::from([("workers", Item { hosts, vars })]);

    let worker_hosts_file = PathBuf::from("k3s/inventory/worker-hosts.yaml");
    if worker_hosts_file.exists() {
        remove_file(worker_hosts_file.as_path()).await?;
    }

    fs::write(
        worker_hosts_file.as_path(),
        serde_yaml::to_string(&worker_hosts)?,
    )
    .await?;

    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }

    command(
        "ansible-playbook",
        &[
            "worker.yaml",
            "--private-key",
            &connect_args.private_key_file,
            "-i",
            "inventory/worker-hosts.yaml",
        ],
        verbose,
        [
            "Setting up worker nodes",
            "Could not setup worker nodes",
            "Worker nodes ready",
        ],
        "k3s",
        env,
    )
    .await
}

async fn setup_platform(
    platform_args: &PlatformArgs,
    cli: &SetupArgs,
    verbose: bool,
) -> Result<PlatformConnectInfo> {
    for p in PLATFORMS {
        if p.name() == platform_args.platform {
            if !cli.only_platform_outputs {
                p.pre_setup(platform_args, verbose).await?;
                p.setup(platform_args, verbose).await?;
            }

            let info = p.platform_info(platform_args, verbose).await?;
            return Ok(PlatformConnectInfo {
                private_key_file: info.ssh_key,
                worker_ips: info.worker_ips,
                master_ip: info.master_ip,
                host_username: platform_args.host_username.clone(),
            });
        }
    }
    bail!(format!("Unknown platform {}", platform_args.platform))
}

pub async fn setup(args: &SetupArgs, cli: &Cli) -> Result<()> {
    let config = parse_config(&cli.file)?;
    let connect_args = match setup_platform(&config.setup, &args, cli.verbose).await {
        Ok(p) => p,
        Err(err) => exit!(err, "Could not setup platform"),
    };
    info!("{connect_args:#?}");

    if connect_args.worker_ips.len() < 1 {
        exit!(
            "Check platform setup output",
            "Need at least two nodes for kubernetes, only got {}",
            connect_args.worker_ips.len()
        );
    }

    setup_master_node(&connect_args, config.kubernetes, config.benchmark.drivers, cli.verbose).await?;
    setup_worker_node(&connect_args, cli.verbose).await
}
