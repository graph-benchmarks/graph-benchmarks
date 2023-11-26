use std::{collections::HashMap, env, time::Instant};

use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::ListParams,
    runtime::{watcher, WatchStreamExt},
    Api, Client, ResourceExt,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    args::Cli,
    common::{command, finish_progress, progress},
    config::parse_config,
    driver_config, exit,
    platforms::{PlatformInfo, PLATFORMS},
};

const ALGORITHMS: &[&str] = &["bfs", "pr", "wcc", "cdlp", "lcc", "sssp"];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriverConfig {
    postgres: PostgresConfig,
    ip: String,
    dataset: String,
    output_path: String,
    algo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostgresConfig {
    ip: String,
    db: String,
    user: String,
    password: String
}

pub async fn run_benchmark(cli: &Cli) -> Result<()> {
    let mut config = parse_config(&cli.file)?;

    let connect_args = 'p: {
        for p in PLATFORMS {
            if p.name() == config.setup.platform {
                break 'p p.platform_info(&config.setup, cli.verbose).await?;
            }
        }
        exit!("", "Unknown platform {}", config.setup.platform)
    };

    config.setup.node_configs.sort_by(|a, b| b.cmp(a));
    for n_nodes in config.setup.node_configs {
        for driver in &config.benchmark.drivers {
            let driver_config = match driver_config::get_driver_config(driver) {
                Some(d) => d,
                None => exit!("", "Could not find driver {}", driver)
            };

            setup_driver(&driver, &connect_args, cli.verbose).await?;
            let service_ip = driver_config.get_service_ip().await?;
            let mut cfg = DriverConfig {
                postgres: PostgresConfig { ip: "postgres".to_owned(), db: "postgres".to_owned(), user: "postgres".to_owned(), password: "graph_benchmarks".to_owned() },
                ip: service_ip,
                dataset: "".into(),
                output_path: "/output".into(),
                algo: "".into(),
            };
            
            for dataset in &config.benchmark.datasets {
                for algorithm in config.benchmark.algorithms.as_ref().unwrap_or(&ALGORITHMS.iter().map(|x| x.to_string()).collect::<Vec<String>>()) {
                    cfg.dataset = dataset.clone();
                    cfg.algo = algorithm.clone();
                    info!("{cfg:#?}");
                }
            }
        }
    }

    Ok(())
}

async fn setup_driver(name: &str, connect_args: &PlatformInfo, verbose: bool) -> Result<()> {
    let mut env = HashMap::from([("ANSIBLE_HOST_KEY_CHECKING", "False")]);
    if verbose {
        env.insert("DEBUG_ANSIBLE", "1");
    }

    command(
        "ansible-playbook",
        &[
            "setup.yaml",
            "--private-key",
            &connect_args.ssh_key,
            "-i",
            "../../k3s/inventory/master-hosts.yaml",
        ],
        verbose,
        [
            &format!("Installing {name}"),
            &format!("Could not install {name}"),
            &format!("Installed {name}"),
        ],
        &format!("drivers/{}", name),
        env,
    )
    .await?;

    env::set_var("KUBECONFIG", "k3s/kube-config");    

    let start = Instant::now();
    let pb = progress(&format!("Waiting for {name} to be ready"));
    let client = Client::try_default().await?;

    let mut ready = false;
    let status_check = |pod: Pod| {
        if let Some(status) = pod.status {
            if let Some(cs) = status.container_statuses {
                if cs.len() > 0 && cs[0].ready {
                    return true;
                }
            }
        }
        false
    };

    let pod_label = driver_config::get_driver_config(name).unwrap().pod_ready_label();

    let pods: Api<Pod> = Api::default_namespaced(client.clone());
    if let Ok(pod_list) = pods
        .list(&ListParams::default().labels(pod_label))
        .await
    {
        for pod in pod_list.items {
            ready = status_check(pod);
            if ready {
                break;
            }
        }
    }

    if !ready {
        let api = Api::<Pod>::default_namespaced(client);
        let wc = watcher::Config::default().labels(pod_label);

        let mut res = watcher(api, wc).applied_objects().default_backoff().boxed();

        while let Ok(Some(p)) = res.try_next().await {
            info!("got status update {} {:#?}", p.name_any(), p.status);
            if status_check(p) {
                break;
            }
        }
    }

    finish_progress(
        "Driver ready",
        &format!("drivers/{name}"),
        start.elapsed(),
        Some(pb),
    );
    Ok(())
}
