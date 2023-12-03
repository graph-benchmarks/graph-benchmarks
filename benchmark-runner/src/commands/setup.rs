use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Result};
use common::{
    command::{command_no_print, command_print, finish_progress, progress},
    config::{parse_config, KubeSetup, PlatformConnectInfo, SetupArgs},
    exit,
};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, remove_file};
use tracing::info;

use crate::args::{self, Cli};

struct ImageConfig<'a> {
    name: &'a str,
    path: &'a str,
}

const STANDARD_IMAGES: &[ImageConfig] = &[
    ImageConfig {
        name: "rsync",
        path: "rsync",
    },
    ImageConfig {
        name: "metrics",
        path: "../metrics",
    },
    ImageConfig {
        name: "graphs",
        path: "../graphs",
    },
];

#[derive(Clone, Debug, Serialize, Deserialize)]
struct K3sRegistry {
    #[serde(skip_serializing_if = "Option::is_none")]
    mirrors: Option<HashMap<String, Endpoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    configs: Option<HashMap<String, RegistryConfig>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RegistryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<RegistryAuth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls: Option<RegistryTls>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RegistryAuth {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RegistryTls {
    #[serde(skip_serializing_if = "Option::is_none")]
    cert_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    insecure_skip_verify: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Endpoint {
    endpoint: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item<'a> {
    pub hosts: HashMap<String, ()>,
    #[serde(borrow)]
    pub vars: HashMap<&'a str, String>,
}

async fn setup_master_node(
    connect_args: &PlatformConnectInfo,
    kube_config: Option<KubeSetup>,
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

    let registry_file = Path::new("k3s/data/k3s_registry_config.yaml");
    if !registry_file.exists() {
        fs::write(registry_file, "").await?;
    }
    let mut registry_cfg: K3sRegistry =
        serde_yaml::from_str(&fs::read_to_string(registry_file).await?)?;
    let mirror = match &mut registry_cfg.mirrors {
        Some(m) => m,
        None => {
            registry_cfg.mirrors = Some(HashMap::new());
            registry_cfg.mirrors.as_mut().unwrap()
        }
    };
    mirror.insert(
        format!("{}:30000", connect_args.master_ip),
        Endpoint {
            endpoint: vec![format!("http://{}:30000", connect_args.master_ip)],
        },
    );
    fs::write(
        "k3s/data/k3s_registry.yaml",
        serde_yaml::to_string(&registry_cfg)?,
    )
    .await?;

    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }
    command_print(
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

    let pb = progress("Building & loading internal benchmark resources");
    let start = Instant::now();
    for image in STANDARD_IMAGES {
        command_no_print(
            "ansible-playbook",
            &[
                "load-image.yaml",
                "--private-key",
                &connect_args.private_key_file,
                "-i",
                "inventory/master-hosts.yaml",
                "--extra-vars",
                &format!(
                    "image_path={} image_name={} repo=system",
                    image.path, image.name
                ),
            ],
            "k3s",
            env.clone(),
        )
        .await?;
    }
    finish_progress(
        "Internal benchmark resources ready",
        "containers",
        start.elapsed(),
        Some(pb),
    );

    for driver in drivers {
        command_print(
            "ansible-playbook",
            &[
                "load-image.yaml",
                "--private-key",
                &connect_args.private_key_file,
                "-i",
                "inventory/master-hosts.yaml",
                "--extra-vars",
                &format!("image_path=../drivers/{driver} image_name={driver} repo=benches"),
            ],
            verbose,
            [
                &format!("Building & loading {driver} driver into kubernetes"),
                &format!("Could not build / load {driver} driver into kubernetes"),
                &format!("Done building & loading {driver} driver into kubernetes"),
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

    command_print(
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
    setup_args: &SetupArgs,
    cli: &args::SetupArgs,
    verbose: bool,
) -> Result<PlatformConnectInfo> {
    for p in base_provider::PROVIDERS {
        if p.name() == setup_args.provider {
            if !cli.only_software_setup {
                p.pre_setup(setup_args, verbose).await?;
                p.setup(setup_args, verbose).await?;
            }

            let info = p.platform_info(setup_args, verbose).await?;
            return Ok(PlatformConnectInfo {
                private_key_file: info.ssh_key,
                worker_ips: info.worker_ips,
                master_ip: info.master_ip,
                host_username: setup_args.host_username.clone(),
            });
        }
    }
    bail!(format!("Unknown platform {}", setup_args.platform))
}

pub async fn setup(args: &args::SetupArgs, cli: &Cli) -> Result<()> {
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

    setup_master_node(
        &connect_args,
        config.kubernetes,
        config.benchmark.drivers,
        cli.verbose,
    )
    .await?;
    setup_worker_node(&connect_args, cli.verbose).await
}
